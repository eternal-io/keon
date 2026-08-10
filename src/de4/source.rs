use super::*;
use core::{cmp::Ordering, num::NonZeroU8};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use lexical_parse_float::FromLexicalWithOptions as _;
use lexical_parse_integer::FromLexicalWithOptions as _;
use lexical_util::NumberFormatBuilder;
use memchr::{memchr, memchr2, memchr3};
use simdutf8::{basic::from_utf8 as decode_utf8_fast, compat::from_utf8 as decode_utf8};

const NUMBER_FORMAT: u128 = NumberFormatBuilder::new()
    .case_sensitive_base_prefix(true)
    .case_sensitive_special(true)
    .no_positive_mantissa_sign(true)
    .required_integer_digits(true)
    .internal_digit_separator(true)
    .trailing_digit_separator(true)
    .consecutive_digit_separator(true)
    .digit_separator(NonZeroU8::new(b'_'))
    .build_strict();

const NUMBER_FORMAT_HEX_NO_PREFIX: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
    .mantissa_radix(16)
    .build_strict();

const NUMBER_FORMAT_HEX: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
    .mantissa_radix(16)
    .base_prefix(NonZeroU8::new(b'x'))
    .build_strict();

const NUMBER_FORMAT_OCT: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
    .mantissa_radix(8)
    .base_prefix(NonZeroU8::new(b'o'))
    .build_strict();

const NUMBER_FORMAT_BIN: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
    .mantissa_radix(2)
    .base_prefix(NonZeroU8::new(b'b'))
    .build_strict();

const PARSE_INTEGER_OPTIONS: lexical_parse_integer::Options = lexical_parse_integer::options::STANDARD;

const PARSE_FLOAT_OPTIONS: lexical_parse_float::Options = lexical_parse_float::OptionsBuilder::new()
    .lossy(false)
    .exponent(b'e')
    .decimal_point(b'.')
    .nan_string(Some(b"NaN"))
    .inf_string(None)
    .infinity_string(Some(b"inf"))
    .build_strict();

pub(super) enum Indicator<'t> {
    Unit,
    Bool(bool),
    Char(char),
    Byte(u8),
    Number(NumberKind),
    String(StringKind),
    Bytes(BytesKind),
    Initiator(Initiator),
    Identifier(Option<&'t Ident>),
    ExplicitNewtype(Option<&'t Ident>),
    ExplicitVariant(Option<&'t Ident>, &'t Ident),
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Initiator {
    /** `?` */ Maybe,
    /** `[` */ Array,
    /** `(` */ Tuple,
    /** `{` */ Map,
    /**`..` */ DotDot,
    /**`..=`*/ DotDotEq,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RangeSeparator {
    /**`..` */ DotDot,
    /**`..=`*/ DotDotEq,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NominalBodyInitiator {
    /** `(` */ Tuple,
    /** `{` */ Struct,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Delimiter {
    /** `]` */ Array,
    /** `)` */ Tuple,
    /** `}` */ MapLike,
    /** `,` */ Comma,
    /** `:` */ Colon,
    /** `=>`*/ FatArrow,
    /** `;` */ SemiColon,
               EOF,
}

pub(super) enum NumberKind {
    Common,
    Infinity,
    NotANumber,
}

pub(super) enum Radix {
    Dec = 10,
    Hex = 16,
    Oct = 8,
    Bin = 2,
}

pub(super) enum StringKind {
    Normal,
    Raw { ticks: usize },
    Paragraph { ticks: usize },
}

pub(super) enum BytesKind {
    Normal,
    Raw { ticks: usize },
    Base64,
    Base32,
    Base16,
}

trait LenUtf8 {
    fn len_utf8(&self) -> usize;
}

impl LenUtf8 for Delimiter {
    fn len_utf8(&self) -> usize {
        match self {
            Delimiter::FatArrow => 2,
            Delimiter::EOF => 0,
            _ => 1,
        }
    }
}

impl LenUtf8 for NumberSuffix {
    fn len_utf8(&self) -> usize {
        match self {
            NumberSuffix::Int8 => 2,
            NumberSuffix::Int16 | NumberSuffix::Int32 | NumberSuffix::Int64 => 3,
            NumberSuffix::Int128 => 4,
            NumberSuffix::UInt8 => 2,
            NumberSuffix::UInt16 | NumberSuffix::UInt32 | NumberSuffix::UInt64 => 3,
            NumberSuffix::UInt128 => 4,
            NumberSuffix::Float32 | NumberSuffix::Float64 => 3,
        }
    }
}

#[expect(private_bounds)]
pub trait Source<'de>: ParseToConcr<'de> + ParseToValue<'de> {}

pub(super) trait ParseHelper<'de> {
    fn position(&self) -> Position;

    /// Skips WS: Consumes the subsequent whitespaces and comments.
    fn eat_ws(&mut self) -> ResultKind;

    /// Skips WS and consumes the specified delimiter if possible:
    /// - returns `None` on success;
    /// - returns `Some` if another delimiter is found;
    /// - returns `Err` if no delimiter is found.
    fn delim(&mut self, delim: Delimiter) -> ResultKind<Option<Delimiter>>;

    /// Skips WS and consumes the specified delimiter. Returns `Err` if not found.
    fn delim_expected(&mut self, delim: Delimiter, reason: ErrorKind) -> ResultKind {
        self.delim(delim)?
            .is_none()
            .then_some(())
            .ok_or(reason)
            .map_err(ErrorImpl::from)
    }

    /// Skips WS and checks the presence of a subsequent delimiter.
    fn adjacent_to_delim(&mut self) -> ResultKind<bool>;

    /// Skips WS and checks the presence of a subsequent delimiter. Returns `Err(reason)` if not found.
    fn adjacent_to_delim_expected(&mut self, reason: ErrorKind) -> ResultKind {
        self.adjacent_to_delim()?
            .then_some(())
            .ok_or(reason)
            .map_err(ErrorImpl::from)
    }

    /// Skips WS and checks whether the subsequent content appears to be a scalar.
    fn adjacent_to_scalar(&mut self) -> ResultKind<bool>;

    /// Skips WS and checks whether the subsequent content appears to be a scalar. Returns `Err` if false.
    fn adjacent_to_scalar_expected(&mut self) -> ResultKind {
        self.adjacent_to_scalar()?
            .then_some(())
            .ok_or(ErrorKind::ExpectedScalar)
            .map_err(ErrorImpl::from)
    }

    fn finish_one(&mut self) -> ResultKind;
    fn finish_all(&mut self) -> ResultKind;
}

pub(super) trait ParseToConcr<'de>: ParseHelper<'de> {
    /// If the subsequent content starts with `b'`, consume it and return true.
    fn try_byte(&mut self) -> ResultKind<bool>;

    fn begin_char(&mut self) -> ResultKind;

    fn begin_string(&mut self) -> ResultKind<StringKind>;

    fn begin_bytes(&mut self) -> ResultKind<BytesKind>;

    fn begin_maybe(&mut self) -> ResultKind;

    fn begin_array(&mut self) -> ResultKind;
    fn end_array(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::Array)? {
            if let Delimiter::Comma = delim {
                raise(ErrorKind::DuplicatedComma)
            } else {
                raise(ErrorKind::ExpectedArrayEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_tuple(&mut self) -> ResultKind;
    fn end_tuple(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::Tuple)? {
            if let Delimiter::Comma = delim {
                raise(ErrorKind::DuplicatedComma)
            } else {
                raise(ErrorKind::ExpectedTupleEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_map_like(&mut self) -> ResultKind;
    fn end_map_like(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::MapLike)? {
            if let Delimiter::Comma = delim {
                raise(ErrorKind::DuplicatedComma)
            } else {
                raise(ErrorKind::ExpectedMapLikeEnd)
            }
        } else {
            Ok(())
        }
    }

    /// Skips WS and consumes the specified range separator if possible,
    /// either `..` (not inclusive) or `..=` (inclusive). Returns true on success.
    fn range_to(&mut self, inclusive: bool) -> ResultKind<bool>;

    /// Skips WS and consumes the subsequent `..`. Returns `Err` if not found.
    fn end_range_from(&mut self) -> ResultKind;

    fn parse_unit(&mut self) -> ResultKind;

    fn parse_bool(&mut self) -> ResultKind<bool>;

    // NOTE: According to the grammar spec, WS is not allowed between negative signs and digits.
    fn parse_i8(&mut self) -> ResultKind<i8>;
    fn parse_i16(&mut self) -> ResultKind<i16>;
    fn parse_i32(&mut self) -> ResultKind<i32>;
    fn parse_i64(&mut self) -> ResultKind<i64>;
    fn parse_i128(&mut self) -> ResultKind<i128>;
    fn parse_u8(&mut self) -> ResultKind<u8>;
    fn parse_u16(&mut self) -> ResultKind<u16>;
    fn parse_u32(&mut self) -> ResultKind<u32>;
    fn parse_u64(&mut self) -> ResultKind<u64>;
    fn parse_u128(&mut self) -> ResultKind<u128>;
    fn parse_f32(&mut self) -> ResultKind<f32>;
    fn parse_f64(&mut self) -> ResultKind<f64>;

    fn parse_byte(&mut self) -> ResultKind<u8>;

    fn parse_char(&mut self) -> ResultKind<char>;

    fn parse_string<'t>(&mut self, kind: StringKind, scratch: &'t mut Vec<u8>)
        -> ResultKind<Either<&'de str, &'t str>>;

    fn parse_bytes<'t>(&mut self, kind: BytesKind, scratch: &'t mut Vec<u8>)
        -> ResultKind<Either<&'de [u8], &'t [u8]>>;

    fn parse_identifier<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<&'t Ident>
    where
        'de: 't;

    fn parse_struct_name<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<Option<&'t Ident>>
    where
        'de: 't;

    fn parse_newtype_name<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<Option<&'t Ident>>
    where
        'de: 't;

    // NOTE: According to the grammar spec, WS is not allowed surrounding path separators.
    fn parse_variant_name<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<(Option<&'t Ident>, &'t Ident)>
    where
        'de: 't;
}

pub(super) trait ParseToValue<'de>: ParseToConcr<'de> {
    fn begin<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<Indicator<'t>>
    where
        'de: 't;

    fn parse_number(&mut self, kind: NumberKind) -> ResultKind<Number2>;

    /// Skips WS and consumes the subsequent range separator. Returns `None` if not found.
    fn range_separator(&mut self) -> ResultKind<Option<RangeSeparator>>;

    /// Skips WS and consumes the subsequent nominal body initiator. Returns `None` if not found.
    fn nominal_body_initiator(&mut self) -> ResultKind<Option<NominalBodyInitiator>>;
}

//==================================================================================================

macro_rules! try_downcast_integer {
    ($expr:expr, $up_ty:ty, $ty:ty) => {{
        let n: $up_ty = $expr;

        if n < <$up_ty>::from(<$ty>::MIN) {
            return raise(ErrorKind::IntegerUnderflow);
        }
        if n > <$up_ty>::from(<$ty>::MAX) {
            return raise(ErrorKind::IntegerOverflow);
        }

        n as $ty
    }};
}

macro_rules! fn_parse_integer_immediate {
    ($name:ident, $ty:ty) => {
        fn $name(bytes: &[u8], radix: Radix) -> ResultKind<$ty> {
            #[cold]
            fn power_of_two(bytes: &[u8], radix: Radix) -> ResultKind<$ty> {
                match radix {
                    Radix::Hex => <$ty>::from_lexical_with_options::<NUMBER_FORMAT_HEX>(bytes, &PARSE_INTEGER_OPTIONS),
                    Radix::Oct => <$ty>::from_lexical_with_options::<NUMBER_FORMAT_OCT>(bytes, &PARSE_INTEGER_OPTIONS),
                    Radix::Bin => <$ty>::from_lexical_with_options::<NUMBER_FORMAT_BIN>(bytes, &PARSE_INTEGER_OPTIONS),
                    Radix::Dec => unsafe { core::hint::unreachable_unchecked() },
                }
                .map_err(ErrorImpl::from)
            }
            match radix {
                Radix::Dec => <$ty>::from_lexical_with_options::<NUMBER_FORMAT>(bytes, &PARSE_INTEGER_OPTIONS)
                    .map_err(ErrorImpl::from),
                radix => power_of_two(bytes, radix),
            }
        }
    };
}

fn_parse_integer_immediate!(parse_i64, i64);
fn_parse_integer_immediate!(parse_i128, i128);
fn_parse_integer_immediate!(parse_u64, u64);
fn_parse_integer_immediate!(parse_u128, u128);
fn parse_f64(bytes: &[u8]) -> ResultKind<f64> {
    f64::from_lexical_with_options::<NUMBER_FORMAT>(bytes, &PARSE_FLOAT_OPTIONS).map_err(ErrorImpl::from)
}

macro_rules! fn_parse_integer {
    ($method:ident, $ty:ty) => {
        #[rustfmt::skip]
        fn $method(&mut self) -> ResultKind<$ty> {
            #[cold]
            fn power_of_two(bytes: &[u8], radix: Radix) -> ResultKind<($ty, usize)> {
                match radix {
                    Radix::Hex => <$ty>::from_lexical_partial_with_options::<NUMBER_FORMAT_HEX>(bytes, &PARSE_INTEGER_OPTIONS),
                    Radix::Oct => <$ty>::from_lexical_partial_with_options::<NUMBER_FORMAT_OCT>(bytes, &PARSE_INTEGER_OPTIONS),
                    Radix::Bin => <$ty>::from_lexical_partial_with_options::<NUMBER_FORMAT_BIN>(bytes, &PARSE_INTEGER_OPTIONS),
                    Radix::Dec => unsafe { core::hint::unreachable_unchecked() },
                }
                .map_err(ErrorImpl::from)
            }
            let (n, len) = match self.peek_integer_radix() {
                Radix::Dec => <$ty>::from_lexical_partial_with_options::<NUMBER_FORMAT>(self.rest(), &PARSE_INTEGER_OPTIONS)
                    .map_err(ErrorImpl::from),
                radix => power_of_two(self.rest(), radix),
            }?;
            self.bump(len);

            Ok(n)
        }
    };
}

macro_rules! fn_parse_integer_case {
    ($method:ident, $ty:ty, $up_method:ident, $up_ty:ty, $suff:ident) => {
        fn $method(&mut self) -> ResultKind<$ty> {
            let n = try_downcast_integer!(self.$up_method()?, $up_ty, $ty);
            self.number_suffix(NumberSuffix::$suff)?;
            Ok(n)
        }
    };
}

macro_rules! fn_parse_float_case {
    ($method:ident, $ty:ty, $suff:ident) => {
        fn $method(&mut self) -> ResultKind<$ty> {
            let f = self.parse_f64()?;
            self.number_suffix(NumberSuffix::$suff)?;
            Ok(f as $ty)
        }
    };
}

pub struct SliceSource<'de> {
    src: &'de [u8],
    idx: usize,
}

impl<'de> SliceSource<'de> {
    fn rest(&self) -> &'de [u8] {
        &self.src[self.idx..]
    }

    fn raise<T>(&mut self, offset: usize, reason: ErrorKind) -> ResultKind<T> {
        self.bump(offset);
        raise(reason)
    }

    fn raise_unexpected_eof<T>(&mut self) -> ResultKind<T> {
        self.bump_to_end();
        raise(ErrorKind::UnexpectedEof)
    }

    fn bump(&mut self, len: usize) {
        debug_assert!(len <= self.rest().len());
        debug_assert!(
            decode_utf8_fast(&self.rest()[..len]).is_ok(),
            "Uncaught invalid UTF-8 sequence!!"
        );
        self.idx += len;
    }

    fn bump_to_end(&mut self) {
        self.idx = self.src.len();
    }

    /// Consumes the subsequent characters that have `Pattern_White_Space` Unicode property.
    fn eat_ws_pure(&mut self) {
        self.bump(self.rest().len() - Self::trim_start(self.rest()).len());
    }

    fn trim_start(mut bytes: &[u8]) -> &[u8] {
        while let
            | [b'\x09'..=b'\x0D', end @ ..]             // 0009..000D <control-0009>..<control-000D>
            | [b'\x20', end @ ..]                       // 0020       SPACE
            | [b'\xC2', b'\x85', end @ ..]              // 0085       <control-0085>
            | [b'\xE2', b'\x80', b'\x8E', end @ ..]     // 200E       LEFT-TO-RIGHT MARK
            | [b'\xE2', b'\x80', b'\x8F', end @ ..]     // 200F       RIGHT-TO-LEFT MARK
            | [b'\xE2', b'\x80', b'\xA8', end @ ..]     // 2028       LINE SEPARATOR
            | [b'\xE2', b'\x80', b'\xA9', end @ ..]     // 2029       PARAGRAPH SEPARATOR
            = bytes { bytes = end }
        bytes
    }

    fn trim_end(mut bytes: &[u8]) -> &[u8] {
        while let
            | [start @ .., b'\x09'..=b'\x0D']           // 0009..000D <control-0009>..<control-000D>
            | [start @ .., b'\x20']                     // 0020       SPACE
            | [start @ .., b'\xC2', b'\x85']            // 0085       <control-0085>
            | [start @ .., b'\xE2', b'\x80', b'\x8E']   // 200E       LEFT-TO-RIGHT MARK
            | [start @ .., b'\xE2', b'\x80', b'\x8F']   // 200F       RIGHT-TO-LEFT MARK
            | [start @ .., b'\xE2', b'\x80', b'\xA8']   // 2028       LINE SEPARATOR
            | [start @ .., b'\xE2', b'\x80', b'\xA9']   // 2029       PARAGRAPH SEPARATOR
            = bytes { bytes = start }
        bytes
    }

    /// Skips WS and peeks the subsequent delimiter.
    fn seek_delim(&mut self) -> ResultKind<Option<Delimiter>> {
        self.eat_ws()?;
        'delim: {
            Ok(Some(match self.rest() {
                [b']', ..] => Delimiter::Array,
                [b')', ..] => Delimiter::Tuple,
                [b'}', ..] => Delimiter::MapLike,
                [b',', ..] => Delimiter::Comma,
                [b':', ..] => Delimiter::Colon,
                [b'=', b'>', ..] => Delimiter::FatArrow,
                [b';', ..] => Delimiter::SemiColon,
                [] => Delimiter::EOF,
                _ => break 'delim Ok(None),
            }))
        }
    }

    fn consume(&mut self, needle: &str) -> bool {
        if self.rest().starts_with(needle.as_bytes()) {
            self.bump(needle.len());
            true
        } else {
            false
        }
    }

    fn consume_expected(&mut self, needle: &str, reason: ErrorKind) -> ResultKind {
        self.consume(needle)
            .then_some(())
            .ok_or(reason)
            .map_err(ErrorImpl::from)
    }

    fn consume_ticks_peek_initiator(&mut self) -> (usize, Option<u8>) {
        let ticks = self.rest().iter().take_while(|&&byte| byte == b'`').count();
        self.bump(ticks);
        if let Some(init) = self.rest().first().copied() {
            if init < 0x80 {
                return (ticks, Some(init));
            }
        }
        (ticks, None)
    }

    fn peek_ticks_from(&self, offset: usize) -> usize {
        self.rest()[offset..].iter().take_while(|&&byte| byte == b'`').count()
    }

    fn peek_integer_radix(&self) -> Radix {
        match self.rest() {
            [b'-', b'0', b'b', ..] | [b'0', b'b', ..] => Radix::Bin,
            [b'-', b'0', b'o', ..] | [b'0', b'o', ..] => Radix::Oct,
            [b'-', b'0', b'x', ..] | [b'0', b'x', ..] => Radix::Hex,
            _ => Radix::Dec,
        }
    }

    fn number_suffix(&mut self, accepted: NumberSuffix) -> ResultKind {
        let suff = 'suff: {
            Some(match self.rest() {
                [b'i', b'8', ..] => NumberSuffix::Int8,
                [b'i', b'1', b'6', ..] => NumberSuffix::Int16,
                [b'i', b'3', b'2', ..] => NumberSuffix::Int32,
                [b'i', b'6', b'4', ..] => NumberSuffix::Int64,
                [b'i', b'1', b'2', b'8', ..] => NumberSuffix::Int128,
                [b'u', b'8', ..] => NumberSuffix::UInt8,
                [b'u', b'1', b'6', ..] => NumberSuffix::UInt16,
                [b'u', b'3', b'2', ..] => NumberSuffix::UInt32,
                [b'u', b'6', b'4', ..] => NumberSuffix::UInt64,
                [b'u', b'1', b'2', b'8', ..] => NumberSuffix::UInt128,
                [b'f', b'3', b'2', ..] => NumberSuffix::Float32,
                [b'f', b'6', b'4', ..] => NumberSuffix::Float64,
                _ => break 'suff None,
            })
        };
        if let Some((ch, _)) = self.decode_from(0)? {
            if unicode_ident::is_xid_continue(ch) {
                return raise(ErrorKind::InvalidNumberSuffix);
            }
        }
        let Some(suff) = suff else {
            return Ok(());
        };
        if core::mem::discriminant(&suff) != core::mem::discriminant(&accepted) {
            return raise(match accepted {
                NumberSuffix::Int8 => ErrorKind::ExpectedInt8,
                NumberSuffix::Int16 => ErrorKind::ExpectedInt16,
                NumberSuffix::Int32 => ErrorKind::ExpectedInt32,
                NumberSuffix::Int64 => ErrorKind::ExpectedInt64,
                NumberSuffix::Int128 => ErrorKind::ExpectedInt128,
                NumberSuffix::UInt8 => ErrorKind::ExpectedUInt8,
                NumberSuffix::UInt16 => ErrorKind::ExpectedUInt16,
                NumberSuffix::UInt32 => ErrorKind::ExpectedUInt32,
                NumberSuffix::UInt64 => ErrorKind::ExpectedUInt64,
                NumberSuffix::UInt128 => ErrorKind::ExpectedUInt128,
                NumberSuffix::Float32 => ErrorKind::ExpectedFloat32,
                NumberSuffix::Float64 => ErrorKind::ExpectedFloat64,
            });
        }
        self.bump(suff.len_utf8());

        Ok(())
    }

    fn_parse_integer!(parse_i64, i64);
    fn_parse_integer!(parse_i128, i128);
    fn_parse_integer!(parse_u64, u64);
    fn_parse_integer!(parse_u128, u128);
    fn parse_f64(&mut self) -> ResultKind<f64> {
        let (f, len) = f64::from_lexical_partial_with_options::<NUMBER_FORMAT>(self.rest(), &PARSE_FLOAT_OPTIONS)?;
        self.bump(len);

        Ok(f)
    }

    fn peek_escape_byte_from(&self, offset: usize) -> ResultKind<(u8, usize)> {
        let byte = match &self.rest()[offset..] {
            [b'\\', ..] => b'\\',
            [b'\"', ..] => b'\"',
            [b'\'', ..] => b'\'',
            [b'0', ..] => b'\0',
            [b'n', ..] => b'\n',
            [b't', ..] => b'\t',
            [b'r', ..] => b'\r',
            [b'x', rest @ ..] => {
                if let Some((bytes, _)) = rest.split_first_chunk() {
                    let byte = Self::parse_u8_fmt_02_hex(bytes)?;

                    return Ok((byte, 1 + 2));
                }
                return raise(ErrorKind::InvalidByteEscape);
            }
            _ => return raise(ErrorKind::InvalidByteEscape),
        };
        Ok((byte, 1))
    }

    fn peek_escape_char_from(&self, offset: usize) -> ResultKind<(char, usize)> {
        let ch = match &self.rest()[offset..] {
            [b'\\', ..] => '\\',
            [b'\"', ..] => '\"',
            [b'\'', ..] => '\'',
            [b'0', ..] => '\0',
            [b'n', ..] => '\n',
            [b't', ..] => '\t',
            [b'r', ..] => '\r',
            [b'x', rest @ ..] => {
                if let Some((bytes, _)) = rest.split_first_chunk() {
                    let byte = Self::parse_u8_fmt_02_hex(bytes)?;
                    if byte < 0x80 {
                        return Ok((byte as char, 1 + 2));
                    }
                }
                return raise(ErrorKind::InvalidAsciiEscape);
            }
            [b'u', rest @ ..] => {
                'unicode: {
                    let Some(b'{') = rest.first() else {
                        break 'unicode;
                    };
                    let Some(len) = rest[1..].iter().position(|&byte| byte == b'}') else {
                        break 'unicode;
                    };
                    let Ok(codep) = u32::from_lexical_with_options::<NUMBER_FORMAT_HEX_NO_PREFIX>(
                        &rest[1..][..len],
                        &PARSE_INTEGER_OPTIONS,
                    ) else {
                        break 'unicode;
                    };
                    let Some(ch) = char::from_u32(codep) else {
                        break 'unicode;
                    };
                    return Ok((ch, 1 + 1 + len + 1));
                }
                return raise(ErrorKind::InvalidUnicodeEscape);
            }
            _ => return raise(ErrorKind::InvalidAsciiEscape),
        };
        Ok((ch, 1))
    }

    fn parse_u8_fmt_02_hex(bytes: &[u8; 2]) -> ResultKind<u8> {
        Ok(u8::from_lexical_with_options::<NUMBER_FORMAT_HEX_NO_PREFIX>(
            bytes,
            &PARSE_INTEGER_OPTIONS,
        )?)
    }

    fn decode_from(&self, offset: usize) -> ResultKind<Option<(char, usize)>> {
        // Copyright (c) 2008-2010 Bjoern Hoehrmann <bjoern@hoehrmann.de>
        // See http://bjoern.hoehrmann.de/utf-8/decoder/dfa/ for details.
        const UTF8_ACCEPT: u32 = 0;
        const UTF8_REJECT: u32 = 12;
        #[rustfmt::skip]
        const UTF8D: [u8; 364] = [
            // The first part of the table maps bytes to character classes that
            // to reduce the size of the transition table and create bitmasks.
             0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
             0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
             0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
             0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
             1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,  9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
             7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,  7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
             8,8,2,2,2,2,2,2,2,2,2,2,2,2,2,2,  2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
            10,3,3,3,3,3,3,3,3,3,3,3,3,4,3,3, 11,6,6,6,5,8,8,8,8,8,8,8,8,8,8,8,
            // The second part is a transition table that maps a combination
            // of a state of the automaton and a character class to a state.
             0,12,24,36,60,96,84,12,12,12,48,72, 12,12,12,12,12,12,12,12,12,12,12,12,
            12, 0,12,12,12,12,12, 0,12, 0,12,12, 12,24,12,12,12,12,12,24,12,24,12,12,
            12,12,12,12,12,12,12,24,12,12,12,12, 12,24,12,12,12,12,12,12,12,24,12,12,
            12,12,12,12,12,12,12,36,12,36,12,12, 12,36,12,12,12,12,12,36,12,36,12,12,
            12,36,12,12,12,12,12,12,12,12,12,12,
        ];

        if self.rest()[offset..].is_empty() {
            return Ok(None);
        }

        let mut state = UTF8_ACCEPT;
        let mut codep = 0;
        for (i, &byte) in self.rest()[offset..].iter().enumerate() {
            let type_ = UTF8D[byte as usize] as u32;

            codep = if state != UTF8_ACCEPT {
                codep << 6 | 0x3F & byte as u32
            } else {
                (0xFF >> type_) & byte as u32
            };
            state = UTF8D[256 + state as usize + type_ as usize] as u32;

            if state == UTF8_ACCEPT {
                return Ok(Some((unsafe { char::from_u32_unchecked(codep) }, i + 1)));
            }
            if state == UTF8_REJECT {
                break;
            }
        }

        raise(ErrorKind::InvalidUtf8Sequence)
    }

    fn decode_expected(&self) -> ResultKind<(char, usize)> {
        self.decode_from(0)?
            .ok_or(ErrorKind::UnexpectedEof)
            .map_err(ErrorImpl::from)
    }

    fn parse_identifier(&mut self) -> ResultKind<&'de Ident> {
        self.parse_identifier_or_underscore()?
            .ok_or(ErrorKind::UnexpectedUnderscoreIdentifier)
            .map_err(ErrorImpl::from)
    }

    fn parse_identifier_or_underscore(&mut self) -> ResultKind<Option<&'de Ident>> {
        if let Some((raw_mode, ident)) = self.parse_identifier_or_underscore_raw()? {
            if !raw_mode && matches!(ident.as_str(), "true" | "false" | "inf" | "NaN") {
                raise(ErrorKind::UnexpectedKeywordAsIdentifier)
            } else {
                self.bump(ident.len());
                Ok(Some(ident))
            }
        } else {
            self.bump(1);
            Ok(None)
        }
    }

    fn parse_identifier_or_underscore_raw(&mut self) -> ResultKind<Option<(bool, &'de Ident)>> {
        let raw_mode = self.consume("`");
        let mut contd = false;
        let mut offset = 0;

        if self.rest().starts_with(b"_") {
            contd = true;
            offset += 1;
        }
        loop {
            if let Some((ch, len)) = self.decode_from(offset)? {
                if !contd && unicode_ident::is_xid_start(ch) || contd && unicode_ident::is_xid_continue(ch) {
                    contd = true;
                    offset += len;
                    continue;
                }
                if offset == 0 {
                    return raise(ErrorKind::ExpectedIdentifier);
                }
                break;
            }
            if offset == 0 {
                return raise(ErrorKind::UnexpectedEof);
            }
            break;
        }

        let ident = unsafe { core::str::from_utf8_unchecked(&self.rest()[..offset]) };
        if matches!(ident, "_") {
            if raw_mode {
                raise(ErrorKind::UnexpectedUnderscoreIdentifier)
            } else {
                Ok(None)
            }
        } else {
            Ok(Some((raw_mode, Ident::new_unchecked(ident))))
        }
    }
}

impl<'de> Source<'de> for SliceSource<'de> {}

impl<'de> ParseHelper<'de> for SliceSource<'de> {
    #[cold]
    fn position(&self) -> Position {
        // SAFETY: The consumed contents are guaranteed to be a valid UTF-8 sequence.
        let consumed = unsafe { core::str::from_utf8_unchecked(&self.src[..self.idx]) };
        let line_start = match memchr::memrchr(b'\n', consumed.as_bytes()) {
            Some(off) => off + 1,
            None => 0,
        };

        Position {
            line: 1 + memchr::memchr_iter(b'\n', consumed.as_bytes()).count(),
            column: 1 + consumed[line_start..].chars().count(),
        }
    }

    fn eat_ws(&mut self) -> ResultKind {
        loop {
            self.eat_ws_pure();
            match self.rest() {
                [b'/', b'/', ..] => {
                    self.bump(2);
                    if let Some(off) = memchr(b'\n', self.rest()) {
                        decode_utf8_fast(&self.rest()[..off])?;
                        self.bump(off + 1);
                    } else {
                        decode_utf8_fast(&self.rest())?;
                        self.bump_to_end();
                    }
                }
                [b'/', b'*', ..] => {
                    self.bump(2);
                    let mut depth = 1usize;
                    while depth > 0 {
                        if let Some(off) = memchr2(b'*', b'/', self.rest()) {
                            decode_utf8_fast(&self.rest()[..off])?;
                            self.bump(off);
                        } else {
                            decode_utf8_fast(&self.rest())?;
                            self.bump_to_end();
                        }
                        match self.rest() {
                            [b'/', b'*', ..] => {
                                self.bump(2);
                                depth += 1;
                            }
                            [b'*', b'/', ..] => {
                                self.bump(2);
                                depth -= 1;
                            }
                            [_, ..] => (),
                            [] => return raise(ErrorKind::UnclosedBlockComment),
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn delim(&mut self, delim: Delimiter) -> ResultKind<Option<Delimiter>> {
        match self.seek_delim()? {
            Some(found) => {
                if found == delim {
                    self.bump(found.len_utf8());

                    Ok(None)
                } else {
                    Ok(Some(found))
                }
            }
            None => raise(ErrorKind::ExpectedDelimiter),
        }
    }

    fn adjacent_to_delim(&mut self) -> ResultKind<bool> {
        Ok(self.seek_delim()?.is_some())
    }

    fn adjacent_to_scalar(&mut self) -> ResultKind<bool> {
        self.eat_ws()?;
        let appear = match self.rest() {
            [b'0'..=b'9' | b'-', ..] => true,
            [b'b', b'\'', ..] => true,
            [b'\'', ..] => true,
            [b'i', b'n', b'f', ..] | [b'N', b'a', b'N', ..] => match self.decode_from(3)? {
                Some((ch, _)) => !unicode_ident::is_xid_continue(ch),
                None => true,
            },
            _ => false,
        };
        Ok(appear)
    }

    fn finish_one(&mut self) -> ResultKind {
        if let Some(delim @ (Delimiter::SemiColon | Delimiter::EOF)) = self.seek_delim()? {
            self.bump(delim.len_utf8());
            Ok(())
        } else {
            raise(ErrorKind::ExpectedSemicolonOrEndOfInput)
        }
    }

    fn finish_all(&mut self) -> ResultKind {
        if let Some(delim @ (Delimiter::SemiColon | Delimiter::EOF)) = self.seek_delim()? {
            self.bump(delim.len_utf8());
            self.delim_expected(Delimiter::EOF, ErrorKind::ExpectedEndOfInput)
        } else {
            raise(ErrorKind::ExpectedSemicolonOrEndOfInput)
        }
    }
}

impl<'de> ParseToConcr<'de> for SliceSource<'de> {
    fn try_byte(&mut self) -> ResultKind<bool> {
        Ok(self.consume("b'"))
    }

    fn begin_char(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected("'", ErrorKind::ExpectedCharacter)
    }

    fn begin_string(&mut self) -> ResultKind<StringKind> {
        self.eat_ws()?;
        let kind = match self.consume_ticks_peek_initiator() {
            (0, Some(b'"')) => StringKind::Normal,
            (ticks @ 1.., Some(b'"')) => StringKind::Raw { ticks },
            (ticks @ 1.., Some(b'|')) => StringKind::Paragraph { ticks },
            _ => return raise(ErrorKind::ExpectedString),
        };
        self.bump(1);
        Ok(kind)
    }

    fn begin_bytes(&mut self) -> ResultKind<BytesKind> {
        self.eat_ws()?;
        if !self.consume("b") {
            return raise(ErrorKind::ExpectedByteString);
        }
        let kind = if let (ticks, Some(b'"')) = self.consume_ticks_peek_initiator() {
            self.bump(1);
            if ticks == 0 {
                BytesKind::Normal
            } else {
                BytesKind::Raw { ticks }
            }
        } else if self.consume("64\"") {
            BytesKind::Base64
        } else if self.consume("32\"") {
            BytesKind::Base32
        } else if self.consume("16\"") {
            BytesKind::Base16
        } else {
            return raise(ErrorKind::ExpectedByteString);
        };
        Ok(kind)
    }

    fn begin_maybe(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected("?", ErrorKind::ExpectedMaybe)
    }

    fn begin_array(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected("[", ErrorKind::ExpectedArray)
    }

    fn begin_tuple(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected("(", ErrorKind::ExpectedTuple)
    }

    fn begin_map_like(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected("{", ErrorKind::ExpectedMapLike)
    }

    fn range_to(&mut self, inclusive: bool) -> ResultKind<bool> {
        self.eat_ws()?;
        if self.consume("..=") {
            match inclusive {
                true => Ok(true),
                false => raise(ErrorKind::ExpectedRangeDotDot),
            }
        } else if self.consume("..") {
            match !inclusive {
                true => Ok(true),
                false => raise(ErrorKind::ExpectedRangeDotDotEq),
            }
        } else {
            Ok(false)
        }
    }

    fn end_range_from(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected("..", ErrorKind::ExpectedRangeDotDot)
    }

    // NOTE: The following methods would not `eat_ws()` at the leading.

    fn parse_unit(&mut self) -> ResultKind {
        self.consume_expected("(", ErrorKind::ExpectedUnit)?;
        self.eat_ws()?;
        self.consume_expected(")", ErrorKind::ExpectedUnitEnd)
    }

    fn parse_bool(&mut self) -> ResultKind<bool> {
        if self.consume("true") {
            Ok(true)
        } else if self.consume("false") {
            Ok(false)
        } else {
            raise(ErrorKind::ExpectedBoolean)
        }
    }

    fn_parse_integer_case!(parse_i8, i8, parse_i64, i64, Int8);
    fn_parse_integer_case!(parse_i16, i16, parse_i64, i64, Int16);
    fn_parse_integer_case!(parse_i32, i32, parse_i64, i64, Int32);
    fn_parse_integer_case!(parse_i64, i64, parse_i64, i64, Int64);
    fn_parse_integer_case!(parse_i128, i128, parse_i128, i128, Int128);
    fn_parse_integer_case!(parse_u8, u8, parse_u64, u64, UInt8);
    fn_parse_integer_case!(parse_u16, u16, parse_u64, u64, UInt16);
    fn_parse_integer_case!(parse_u32, u32, parse_u64, u64, UInt32);
    fn_parse_integer_case!(parse_u64, u64, parse_u64, u64, UInt64);
    fn_parse_integer_case!(parse_u128, u128, parse_u128, u128, UInt128);
    fn_parse_float_case!(parse_f32, f32, Float32);
    fn_parse_float_case!(parse_f64, f64, Float64);

    fn parse_byte(&mut self) -> ResultKind<u8> {
        let (byte, len) = if self.consume("\\") {
            self.peek_escape_byte_from(0)?
        } else {
            let (ch, len) = self.decode_expected()?;
            if matches!(ch, '\n' | '\t' | '\r') {
                return raise(ErrorKind::UnexpectedControlCharacter);
            }
            if !ch.is_ascii() {
                return raise(ErrorKind::UnexpectedNonAsciiCharacter);
            }
            (ch as u8, len)
        };

        self.bump(len);
        self.consume_expected("'", ErrorKind::ExpectedUnquote)?;
        Ok(byte)
    }

    fn parse_char(&mut self) -> ResultKind<char> {
        let (ch, len) = if self.consume("\\") {
            self.peek_escape_char_from(0)?
        } else {
            let (ch, len) = self.decode_expected()?;
            if matches!(ch, '\n' | '\t' | '\r') {
                return raise(ErrorKind::UnexpectedControlCharacter);
            }
            (ch, len)
        };

        self.bump(len);
        self.consume_expected("'", ErrorKind::ExpectedUnquote)?;
        Ok(ch)
    }

    fn parse_string<'t>(
        &mut self,
        kind: StringKind,
        scratch: &'t mut Vec<u8>,
    ) -> ResultKind<Either<&'de str, &'t str>> {
        scratch.clear();
        let mut buf = [0; 4];
        let mut offset = 0;
        let mut scratched = false;
        match kind {
            StringKind::Normal => {
                loop {
                    let Some(off) = memchr3(b'\\', b'\r', b'\"', &self.rest()[offset..]) else {
                        return self.raise_unexpected_eof();
                    };
                    let frag = &self.rest()[offset..][..off];
                    offset += off;

                    match self.rest()[offset] {
                        b'\\' => {
                            let (ch, len) = self
                                .peek_escape_char_from(offset + 1)
                                .inspect_err(|_| self.bump(offset + 1))?;
                            scratched = true;
                            scratch.extend_from_slice(frag);
                            scratch.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                            offset += 1 + len
                        }
                        b'\r' => {
                            let Some(b'\n') = self.rest().get(offset + 1) else {
                                return self.raise(offset, ErrorKind::UnexpectedCarriageReturn);
                            };
                            scratched = true;
                            scratch.extend_from_slice(frag);
                            scratch.push(b'\n');
                            offset += 1 + 1;
                        }
                        b'\"' => {
                            if scratched {
                                scratch.extend_from_slice(frag);
                            }
                            break;
                        }
                        _ => unreachable!(),
                    }
                }

                let content = if !scratched {
                    Either::Left(decode_utf8(&self.rest()[..offset]).map_err(|e| {
                        self.bump(e.valid_up_to());
                        ErrorKind::InvalidUtf8Sequence
                    })?)
                } else {
                    Either::Right(decode_utf8(scratch).map_err(|e| {
                        self.bump(e.valid_up_to());
                        ErrorKind::InvalidUtf8Sequence
                    })?)
                };

                self.bump(offset + 1);
                Ok(content)
            }

            StringKind::Raw { ticks } => {
                loop {
                    let Some(off) = memchr2(b'\r', b'\"', &self.rest()[offset..]) else {
                        return self.raise_unexpected_eof();
                    };
                    let frag = &self.rest()[offset..][..off];
                    offset += off;

                    match self.rest()[offset] {
                        b'\r' => {
                            let Some(b'\n') = self.rest().get(offset + 1) else {
                                return self.raise(offset, ErrorKind::UnexpectedCarriageReturn);
                            };
                            scratched = true;
                            scratch.extend_from_slice(frag);
                            scratch.push(b'\n');
                            offset += 1 + 1;
                        }
                        b'\"' => {
                            let r_ticks = self.peek_ticks_from(offset + 1);
                            match ticks.cmp(&r_ticks) {
                                Ordering::Greater => offset += 1 + r_ticks,
                                Ordering::Equal => {
                                    if scratched {
                                        scratch.extend_from_slice(frag);
                                    }
                                    break;
                                }
                                Ordering::Less => return self.raise(offset, ErrorKind::UnbalancedRawTicks),
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                let content = if !scratched {
                    Either::Left(decode_utf8(&self.rest()[..offset]).map_err(|e| {
                        self.bump(e.valid_up_to());
                        ErrorKind::InvalidUtf8Sequence
                    })?)
                } else {
                    Either::Right(decode_utf8(scratch).map_err(|e| {
                        self.bump(e.valid_up_to());
                        ErrorKind::InvalidUtf8Sequence
                    })?)
                };

                self.bump(offset + 1 + ticks);
                Ok(content)
            }

            StringKind::Paragraph { ticks } => {
                let mut line_type = b'|';
                let mut prev_line = None;
                let mut par_break = false;
                loop {
                    self.consume(" ");
                    let offset = memchr2(b'\r', b'\n', self.rest());
                    let line = Self::trim_end(match offset {
                        Some(off) => &self.rest()[..off],
                        None => self.rest(),
                    });

                    if let Some(off) = offset {
                        if let Some(b'\r') = self.rest().get(off) {
                            let Some(b'\n') = self.rest().get(off + 1) else {
                                return self.raise(off, ErrorKind::UnexpectedCarriageReturn);
                            };
                            self.bump(off + 1 + 1);
                        } else {
                            self.bump(off + 1);
                        }
                    } else {
                        self.bump_to_end();
                    }

                    'stage_line: {
                        if !scratched {
                            if let Some(first_line) = prev_line {
                                scratched = true;
                                scratch.extend_from_slice(first_line);
                            } else {
                                break 'stage_line;
                            }
                        }
                        match line_type {
                            b'|' => scratch.push(b'\n'),
                            b'<' => {
                                if !scratch.is_empty() && !line.is_empty() {
                                    if par_break {
                                        scratch.push(b'\n');
                                    }
                                }
                            }
                            b'>' => {
                                if !scratch.is_empty() && !line.is_empty() {
                                    if par_break {
                                        scratch.push(b'\n');
                                    } else {
                                        scratch.push(b' ');
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                        scratch.extend_from_slice(line);
                    }

                    if line.is_empty() {
                        par_break = true;
                    } else {
                        par_break = false;
                    }
                    prev_line = Some(line);

                    if self.adjacent_to_delim()? {
                        break;
                    }
                    let (r_ticks, Some(init @ (b'|' | b'<' | b'>'))) = self.consume_ticks_peek_initiator() else {
                        return raise(ErrorKind::InvalidParagraphLineInitiator);
                    };
                    if ticks != r_ticks {
                        return raise(ErrorKind::UnbalancedRawTicks);
                    }
                    self.bump(1);
                    line_type = init;
                }

                let content = if !scratched {
                    Either::Left(decode_utf8(prev_line.unwrap()).map_err(|e| {
                        self.bump(e.valid_up_to());
                        ErrorKind::InvalidUtf8Sequence
                    })?)
                } else {
                    Either::Right(decode_utf8(scratch).map_err(|e| {
                        self.bump(e.valid_up_to());
                        ErrorKind::InvalidUtf8Sequence
                    })?)
                };

                Ok(content)
            }
        }
    }

    fn parse_bytes<'t>(
        &mut self,
        kind: BytesKind,
        scratch: &'t mut Vec<u8>,
    ) -> ResultKind<Either<&'de [u8], &'t [u8]>> {
        let mut offset = 0;
        let mut scratched = false;
        match kind {
            BytesKind::Normal => {
                scratch.clear();
                loop {
                    let Some(off) = memchr3(b'\\', b'\r', b'\"', &self.rest()[offset..]) else {
                        return self.raise_unexpected_eof();
                    };
                    let frag = &self.rest()[offset..][..off];
                    offset += off;

                    match self.rest()[offset] {
                        b'\\' => {
                            let (byte, len) = self
                                .peek_escape_byte_from(offset + 1)
                                .inspect_err(|_| self.bump(offset + 1))?;
                            scratched = true;
                            scratch.extend_from_slice(frag);
                            scratch.push(byte);
                            offset += 1 + len
                        }
                        b'\r' => {
                            let Some(b'\n') = self.rest().get(offset + 1) else {
                                return self.raise(offset, ErrorKind::UnexpectedCarriageReturn);
                            };
                            scratched = true;
                            scratch.extend_from_slice(frag);
                            scratch.push(b'\n');
                            offset += 1 + 1;
                        }
                        b'\"' => {
                            if scratched {
                                scratch.extend_from_slice(frag);
                            }
                            break;
                        }
                        _ => unreachable!(),
                    }
                }

                let content = if !scratched {
                    Either::Left(&self.rest()[..offset])
                } else {
                    Either::Right(&scratch[..])
                };

                self.bump(offset + 1);
                Ok(content)
            }

            BytesKind::Raw { ticks } => {
                scratch.clear();
                loop {
                    let Some(off) = memchr2(b'\r', b'\"', &self.rest()[offset..]) else {
                        return self.raise_unexpected_eof();
                    };
                    let frag = &self.rest()[offset..][..off];
                    offset += off;

                    match self.rest()[offset] {
                        b'\r' => {
                            let Some(b'\n') = self.rest().get(offset + 1) else {
                                return self.raise(offset, ErrorKind::UnexpectedCarriageReturn);
                            };
                            scratched = true;
                            scratch.extend_from_slice(frag);
                            scratch.push(b'\n');
                            offset += 1 + 1;
                        }
                        b'\"' => {
                            let r_ticks = self.peek_ticks_from(offset + 1);
                            match ticks.cmp(&r_ticks) {
                                Ordering::Greater => offset += 1 + r_ticks,
                                Ordering::Equal => {
                                    if scratched {
                                        scratch.extend_from_slice(frag);
                                    }
                                    break;
                                }
                                Ordering::Less => return self.raise(offset, ErrorKind::UnbalancedRawTicks),
                            }
                        }
                        _ => unreachable!(),
                    }
                }

                let content = if !scratched {
                    Either::Left(&self.rest()[..offset])
                } else {
                    Either::Right(&scratch[..])
                };

                self.bump(offset + 1 + ticks);
                Ok(content)
            }

            _ => {
                let encoding = match kind {
                    BytesKind::Base64 => BASE64URL_NOPAD,
                    BytesKind::Base32 => BASE32_NOPAD,
                    BytesKind::Base16 => HEXUPPER_PERMISSIVE,
                    _ => unreachable!(),
                };
                let Some(offset) = memchr(b'"', self.rest()) else {
                    return self.raise_unexpected_eof();
                };
                let encoded = &self.rest()[..offset];

                let new_len = encoding.decode_len(offset).map_err(|e| {
                    self.bump(e.position);
                    e.kind
                })?;
                scratch.resize(new_len, 0);

                let len = encoding.decode_mut(encoded, scratch).map_err(
                    |data_encoding::DecodePartial { error: e, .. }| {
                        self.bump(e.position);
                        e.kind
                    },
                )?;
                scratch.truncate(len);

                self.bump(offset + 1);
                Ok(Either::Right(scratch))
            }
        }
    }

    fn parse_identifier<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<&'t Ident>
    where
        'de: 't,
    {
        let _ = scratch;
        self.parse_identifier()
    }

    fn parse_struct_name<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<Option<&'t Ident>>
    where
        'de: 't,
    {
        let _ = scratch;
        self.parse_identifier_or_underscore()
    }

    fn parse_newtype_name<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<Option<&'t Ident>>
    where
        'de: 't,
    {
        let _ = scratch;
        if self.consume("~") {
            self.parse_identifier().map(Some)
        } else {
            self.consume("!");
            Ok(None)
        }
    }

    fn parse_variant_name<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<(Option<&'t Ident>, &'t Ident)>
    where
        'de: 't,
    {
        let _ = scratch;
        if self.consume(".") {
            Ok((None, self.parse_identifier()?))
        } else {
            let name_or_variant = self.parse_identifier()?;
            if self.consume("::") {
                Ok((Some(name_or_variant), self.parse_identifier()?))
            } else {
                Ok((None, name_or_variant))
            }
        }
    }
}

impl<'de> ParseToValue<'de> for SliceSource<'de> {
    fn begin<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<Indicator<'t>>
    where
        'de: 't,
    {
        let _ = scratch;
        self.eat_ws()?;
        let indicator = if self.consume("~") {
            Indicator::ExplicitNewtype(Some(self.parse_identifier()?))
        } else if self.consume("!") {
            Indicator::ExplicitNewtype(None)
        } else if self.consume("?") {
            Indicator::Initiator(Initiator::Maybe)
        } else if self.consume("(") {
            self.eat_ws()?;
            if self.consume(")") {
                Indicator::Unit
            } else {
                Indicator::Initiator(Initiator::Tuple)
            }
        } else if self.consume("[") {
            Indicator::Initiator(Initiator::Array)
        } else if self.consume("{") {
            Indicator::Initiator(Initiator::Map)
        } else if self.consume("..=") {
            Indicator::Initiator(Initiator::DotDotEq)
        } else if self.consume("..") {
            Indicator::Initiator(Initiator::DotDot)
        } else if self.consume(".") {
            Indicator::ExplicitVariant(None, self.parse_identifier()?)
        } else {
            match self.rest() {
                [b'\'', ..] => {
                    self.bump(1);
                    Indicator::Char(self.parse_char()?)
                }
                [b'b', b'\'', ..] => {
                    self.bump(2);
                    Indicator::Byte(self.parse_byte()?)
                }
                [b'0'..=b'9' | b'-', ..] => Indicator::Number(NumberKind::Common),

                /* String */
                [b'"', ..] | [b'`', b'`' | b'"' | b'|', ..] => {
                    let (ticks, init) = self.consume_ticks_peek_initiator();
                    if let Some(b'"') = init {
                        self.bump(1);
                        match ticks {
                            0 => Indicator::String(StringKind::Normal),
                            _ => Indicator::String(StringKind::Raw { ticks }),
                        }
                    } else if let Some(b'|') = init {
                        self.bump(1);
                        Indicator::String(StringKind::Paragraph { ticks })
                    } else {
                        return raise(ErrorKind::ExpectedQuote);
                    }
                }

                /* Byte String */
                [b'b', b'"' | b'`', ..] => {
                    self.bump(1);
                    let (ticks, init) = self.consume_ticks_peek_initiator();
                    if let Some(b'"') = init {
                        self.bump(1);
                        match ticks {
                            0 => Indicator::Bytes(BytesKind::Normal),
                            _ => Indicator::Bytes(BytesKind::Raw { ticks }),
                        }
                    } else {
                        return raise(ErrorKind::ExpectedQuote);
                    }
                }
                [b'b', b'6', b'4', b'"', ..] => {
                    self.bump(4);
                    Indicator::Bytes(BytesKind::Base64)
                }
                [b'b', b'3', b'2', b'"', ..] => {
                    self.bump(4);
                    Indicator::Bytes(BytesKind::Base32)
                }
                [b'b', b'1', b'6', b'"', ..] => {
                    self.bump(4);
                    Indicator::Bytes(BytesKind::Base16)
                }

                _ => 'nominal: {
                    if let Some((raw_mode, name_or_variant)) = self.parse_identifier_or_underscore_raw()? {
                        self.bump(name_or_variant.len());
                        if !raw_mode {
                            match name_or_variant.as_str() {
                                "true" => break 'nominal Indicator::Bool(true),
                                "false" => break 'nominal Indicator::Bool(false),
                                "inf" => break 'nominal Indicator::Number(NumberKind::Infinity),
                                "NaN" => break 'nominal Indicator::Number(NumberKind::NotANumber),
                                _ => (),
                            }
                        }
                        if self.consume("::") {
                            Indicator::ExplicitVariant(Some(name_or_variant), self.parse_identifier()?)
                        } else {
                            Indicator::Identifier(Some(name_or_variant))
                        }
                    } else {
                        self.bump(1);
                        Indicator::Identifier(None)
                    }
                }
            }
        };
        Ok(indicator)
    }

    fn parse_number(&mut self, kind: NumberKind) -> ResultKind<Number2> {
        const BIN_DIGIT: fn(&&u8) -> bool = |byte| matches!(byte, b'0'..=b'1' | b'_');
        const OCT_DIGIT: fn(&&u8) -> bool = |byte| matches!(byte, b'0'..=b'7' | b'_');
        const HEX_DIGIT: fn(&&u8) -> bool = |byte| matches!(byte, b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' | b'_');
        const DEC_FLOAT: fn(&&u8) -> bool =
            |byte| matches!(byte, b'0'..=b'9' | b'.' | b'E' | b'e' | b'+' | b'-' | b'_');

        let num = match kind {
            NumberKind::Common => {
                let special = 'common: {
                    let (off, radix, pred) = match self.rest() {
                        [b'0', b'b', ..] => (2, Radix::Bin, BIN_DIGIT),
                        [b'0', b'o', ..] => (2, Radix::Oct, OCT_DIGIT),
                        [b'0', b'x', ..] => (2, Radix::Hex, HEX_DIGIT),
                        [b'-', b'0', b'b', ..] => (3, Radix::Bin, BIN_DIGIT),
                        [b'-', b'0', b'o', ..] => (3, Radix::Oct, OCT_DIGIT),
                        [b'-', b'0', b'x', ..] => (3, Radix::Hex, HEX_DIGIT),
                        [b'-', b'i', b'n', b'f', ..] => break 'common f64::NEG_INFINITY,
                        [b'-', b'N', b'a', b'N', ..] => break 'common f64::NAN,
                        [b'-', ..] => (1, Radix::Dec, DEC_FLOAT),
                        [..] => (0, Radix::Dec, DEC_FLOAT),
                    };
                    let len = off + self.rest()[off..].iter().take_while(pred).count();
                    let payload = &self.rest()[..len];
                    let suff = 'suff: {
                        Some(match &self.rest()[len..] {
                            [b'i', b'8', ..] => NumberSuffix::Int8,
                            [b'i', b'1', b'6', ..] => NumberSuffix::Int16,
                            [b'i', b'3', b'2', ..] => NumberSuffix::Int32,
                            [b'i', b'6', b'4', ..] => NumberSuffix::Int64,
                            [b'i', b'1', b'2', b'8', ..] => NumberSuffix::Int128,
                            [b'u', b'8', ..] => NumberSuffix::UInt8,
                            [b'u', b'1', b'6', ..] => NumberSuffix::UInt16,
                            [b'u', b'3', b'2', ..] => NumberSuffix::UInt32,
                            [b'u', b'6', b'4', ..] => NumberSuffix::UInt64,
                            [b'u', b'1', b'2', b'8', ..] => NumberSuffix::UInt128,
                            [b'f', b'3', b'2', ..] => NumberSuffix::Float32,
                            [b'f', b'6', b'4', ..] => NumberSuffix::Float64,
                            _ => break 'suff None,
                        })
                    };
                    if let Some((ch, _)) = self.decode_from(len)? {
                        if unicode_ident::is_xid_continue(ch) {
                            return raise(ErrorKind::InvalidNumberSuffix);
                        }
                    }
                    let num = if let Some(ref suff) = suff {
                        match suff {
                            NumberSuffix::Int8 => try_downcast_integer!(parse_i64(payload, radix)?, i64, i8).into(),
                            NumberSuffix::Int16 => try_downcast_integer!(parse_i64(payload, radix)?, i64, i16).into(),
                            NumberSuffix::Int32 => try_downcast_integer!(parse_i64(payload, radix)?, i64, i32).into(),
                            NumberSuffix::Int64 => parse_i64(payload, radix)?.into(),
                            NumberSuffix::Int128 => parse_i128(payload, radix)?.into(),
                            NumberSuffix::UInt8 => try_downcast_integer!(parse_u64(payload, radix)?, u64, u8).into(),
                            NumberSuffix::UInt16 => try_downcast_integer!(parse_u64(payload, radix)?, u64, u16).into(),
                            NumberSuffix::UInt32 => try_downcast_integer!(parse_u64(payload, radix)?, u64, u32).into(),
                            NumberSuffix::UInt64 => parse_u64(payload, radix)?.into(),
                            NumberSuffix::UInt128 => parse_u128(payload, radix)?.into(),
                            NumberSuffix::Float32 => (parse_f64(payload)? as f32).into(),
                            NumberSuffix::Float64 => parse_f64(payload)?.into(),
                        }
                    } else {
                        if memchr3(b'.', b'e', b'E', payload).is_some() {
                            Number2::FloatNoSuffix(parse_f64(payload)?.into())
                        } else if off == 0 {
                            Number2::UIntNoSuffix(parse_u64(payload, radix)?)
                        } else {
                            Number2::IntNoSuffix(parse_i64(payload, radix)?)
                        }
                    };
                    self.bump(len + suff.map(|s| s.len_utf8()).unwrap_or(0));

                    return Ok(num);
                };
                if let Some((ch, _)) = self.decode_from(4)? {
                    if unicode_ident::is_xid_continue(ch) {
                        return raise(ErrorKind::InvalidNumberSpecial);
                    }
                }
                self.bump(4);

                Number2::FloatNoSuffix(special.into())
            }
            NumberKind::Infinity => Number2::FloatNoSuffix(f64::INFINITY.into()),
            NumberKind::NotANumber => Number2::FloatNoSuffix(f64::NAN.into()),
        };
        Ok(num)
    }

    fn range_separator(&mut self) -> ResultKind<Option<RangeSeparator>> {
        if self.consume("..=") {
            Ok(Some(RangeSeparator::DotDotEq))
        } else if self.consume("..") {
            Ok(Some(RangeSeparator::DotDot))
        } else {
            Ok(None)
        }
    }

    fn nominal_body_initiator(&mut self) -> ResultKind<Option<NominalBodyInitiator>> {
        if self.consume("(") {
            Ok(Some(NominalBodyInitiator::Tuple))
        } else if self.consume("{") {
            Ok(Some(NominalBodyInitiator::Struct))
        } else {
            Ok(None)
        }
    }
}
