use super::*;
use core::cmp::Ordering;
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use memchr::{memchr, memchr2, memchr3};
use simdutf8::compat::from_utf8 as decode_utf8;

pub(crate) enum Indicator<'de> {
    Unit,
    Bool(bool),
    Char(char),
    Byte(u8),
    String(StringKind),
    Bytes(BytesKind),
    Number(NumberKind),
    Initiator(Initiator),
    NominalPath(NominalPathRef<'de>),
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Initiator {
    /** `?` */ Maybe,
    /** `[` */ Array,
    /** `(` */ Tuple,
    /** `{` */ MapLike,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delimiter {
    /** `]` */ Array,
    /** `)` */ Tuple,
    /** `}` */ MapLike,
    /** `,` */ Comma,
    /** `:` */ Colon,
    /** `=>`*/ FatArrow,
    /** `;` */ SemiColon,
               EOF,
}

pub(crate) enum NumberSuffix {
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    Float32,
    Float64,
}

pub(crate) enum NumberKind {
    Normal,
    Infinity,
    NegInfinity,
    NotANumber,
}

pub(crate) enum IntegerKind {
    Dec = 10,
    Hex = 16,
    Oct = 8,
    Bin = 2,
}

pub(crate) enum StringKind {
    Normal,
    Raw { ticks: usize },
    Paragraph { ticks: usize },
}

pub(crate) enum BytesKind {
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

pub(crate) trait ParseHelper<'de> {
    /// Position after the last call to `eat_ws()` or `raise()`.
    ///
    /// Note that many methods would call `eat_ws()` implicitly.
    fn position(&self) -> Position;

    /// Consumes the subsequent whitespaces and comments.
    fn eat_ws(&mut self) -> ResultKind;

    /// Skips WS and consumes the specified delimiter if possible:
    /// - returns `None` on success;
    /// - returns `Some` if another delimiter is found;
    /// - returns `Err` if no delimiter is found.
    fn delim(&mut self, delim: Delimiter) -> ResultKind<Option<Delimiter>>;

    /// Skips WS and consumes the specified delimiter. Returns `Err` if not found.
    fn delim_expected(&mut self, delim: Delimiter, reason: ErrorKind) -> ResultKind {
        self.delim(delim)?.is_none().then_some(()).ok_or(reason)
    }

    /// Skips WS and checks the presence of a subsequent delimiter.
    fn adjacent_to_delim(&mut self) -> ResultKind<bool>;

    /// Skips WS and checks the presence of a subsequent delimiter. Returns `Err(reason)` if not found.
    fn adjacent_to_delim_expected(&mut self, reason: ErrorKind) -> ResultKind {
        self.adjacent_to_delim()?.then_some(()).ok_or(reason)
    }
}

pub(crate) trait ParseToConcr<'de>: ParseHelper<'de> {
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
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedArrayEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_tuple(&mut self) -> ResultKind;
    fn end_tuple(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::Tuple)? {
            if let Delimiter::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedTupleEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_map_like(&mut self) -> ResultKind;
    fn end_map_like(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::MapLike)? {
            if let Delimiter::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedMapLikeEnd)
            }
        } else {
            Ok(())
        }
    }

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

    // NOTE: According to the grammar spec, WS is not allowed surrounding path separators.
    fn parse_nominal_path<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<NominalPathRef<'t>>
    where
        'de: 't;

    fn parse_identifier<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<IdentRef<'t>>
    where
        'de: 't;
}

pub(crate) trait ParseToValue<'de>: ParseHelper<'de> {
    fn begin(&mut self) -> ResultKind<Indicator<'de>>;

    /// Skips WS and consumes the subsequent initiator. Returns `None` if not found.
    fn initiator(&mut self) -> ResultKind<Option<Initiator>>;

    fn parse_number(&mut self, kind: NumberKind) -> ResultKind<Either<Number2, NumberNoSuffix2>>;
}

//==================================================================================================

macro_rules! fn_parse_integer {
    ($ty:ty, $name:ident, $number_suffix:path, $error_kind:path) => {
        fn $name(&mut self) -> ResultKind<$ty> {
            use lexical_core::parse_partial_with_options as parse;

            let (n, len) = match self.peek_integer_kind() {
                IntegerKind::Dec => parse::<$ty, NUMBER_FORMAT>(self.rest(), &PARSE_INTEGER_OPTS),
                IntegerKind::Hex => parse::<$ty, NUMBER_FORMAT_HEX>(self.rest(), &PARSE_INTEGER_OPTS),
                IntegerKind::Oct => parse::<$ty, NUMBER_FORMAT_OCT>(self.rest(), &PARSE_INTEGER_OPTS),
                IntegerKind::Bin => parse::<$ty, NUMBER_FORMAT_BIN>(self.rest(), &PARSE_INTEGER_OPTS),
            }?;
            self.bump(len);

            match self.peek_number_suffix() {
                None => self.adjacent_to_delim_expected(ErrorKind::InvalidNumberSuffix)?,
                Some(suffix) => {
                    if !matches!(suffix, $number_suffix) {
                        return Err($error_kind);
                    }
                }
            }
            self.bump($number_suffix.len_utf8());

            Ok(n)
        }
    };
}

macro_rules! fn_parse_float {
    ($ty:ty, $name:ident, $number_suffix:path, $error_kind:path) => {
        fn $name(&mut self) -> ResultKind<$ty> {
            let (f, len) =
                lexical_core::parse_partial_with_options::<$ty, NUMBER_FORMAT>(self.rest(), &PARSE_FLOAT_OPTS)?;
            self.bump(len);

            match self.peek_number_suffix() {
                None => self.adjacent_to_delim_expected(ErrorKind::InvalidNumberSuffix)?,
                Some(suffix) => {
                    if !matches!(suffix, $number_suffix) {
                        return Err($error_kind);
                    }
                }
            }
            self.bump($number_suffix.len_utf8());

            Ok(f)
        }
    };
}

pub struct SliceSource<'de> {
    src: &'de [u8],
    idx: usize,
    report_idx: usize,
}

impl<'de> SliceSource<'de> {
    fn rest(&self) -> &'de [u8] {
        &self.src[self.idx..]
    }

    fn raise<T>(&mut self, offset: usize, reason: ErrorKind) -> ResultKind<T> {
        self.bump(offset);
        self.set_position();
        Err(reason)
    }

    fn raise_unexpected_eof<T>(&mut self) -> ResultKind<T> {
        self.bump_to_end();
        Err(ErrorKind::UnexpectedEof)
    }

    fn bump(&mut self, len: usize) {
        debug_assert!(len <= self.rest().len());
        self.idx += len;
    }

    fn bump_to_end(&mut self) {
        self.idx = self.src.len();
    }

    fn set_position(&mut self) {
        self.report_idx = self.idx;
    }

    /// Consumes the subsequent whitespaces. Refer to [`char::is_whitespace`].
    fn eat_ws_pure(&mut self) {
        loop {
            let len = match self.rest() {
                [b'\x09'..=b'\x0D', ..] => 1,                   // 0009..000D <control-0009>..<control-000D>
                [b'\x20', ..] => 1,                             // 0020       SPACE
                [b'\xC2', b'\x85', ..] => 2,                    // 0085       <control-0085>
                [b'\xC2', b'\xA0', ..] => 2,                    // 00A0       NO-BREAK SPACE
                [b'\xE1', b'\x9A', b'\x80', ..] => 3,           // 1680       OGHAM SPACE MARK
                [b'\xE2', b'\x80', b'\x80'..=b'\x8A', ..] => 3, // 2000..200A EN QUAD..HAIR SPACE
                [b'\xE2', b'\x80', b'\xA8', ..] => 3,           // 2028       LINE SEPARATOR
                [b'\xE2', b'\x80', b'\xA9', ..] => 3,           // 2029       PARAGRAPH SEPARATOR
                [b'\xE2', b'\x80', b'\xAF', ..] => 3,           // 202F       NARROW NO-BREAK SPACE
                [b'\xE2', b'\x81', b'\x9F', ..] => 3,           // 205F       MEDIUM MATHEMATICAL SPACE
                [b'\xE3', b'\x80', b'\x80', ..] => 3,           // 3000       IDEOGRAPHIC SPACE
                _ => break,
            };
            self.bump(len);
        }
    }

    fn trim_end(mut bytes: &[u8]) -> &[u8] {
        while let
            | [left @ .., b'\x09'..=b'\x0D']                    // 0009..000D <control-0009>..<control-000D>
            | [left @ .., b'\x20']                              // 0020       SPACE
            | [left @ .., b'\xC2', b'\x85']                     // 0085       <control-0085>
            | [left @ .., b'\xC2', b'\xA0']                     // 00A0       NO-BREAK SPACE
            | [left @ .., b'\xE1', b'\x9A', b'\x80']            // 1680       OGHAM SPACE MARK
            | [left @ .., b'\xE2', b'\x80', b'\x80'..=b'\x8A']  // 2000..200A EN QUAD..HAIR SPACE
            | [left @ .., b'\xE2', b'\x80', b'\xA8']            // 2028       LINE SEPARATOR
            | [left @ .., b'\xE2', b'\x80', b'\xA9']            // 2029       PARAGRAPH SEPARATOR
            | [left @ .., b'\xE2', b'\x80', b'\xAF']            // 202F       NARROW NO-BREAK SPACE
            | [left @ .., b'\xE2', b'\x81', b'\x9F']            // 205F       MEDIUM MATHEMATICAL SPACE
            | [left @ .., b'\xE3', b'\x80', b'\x80']            // 3000       IDEOGRAPHIC SPACE
            = bytes { bytes = left }
        bytes
    }

    /// Skips WS and peeks the subsequent delimiter.
    fn seek_delim(&mut self) -> ResultKind<Option<Delimiter>> {
        self.eat_ws()?;
        'found: {
            let found = match self.rest() {
                [b']', ..] => Delimiter::Array,
                [b')', ..] => Delimiter::Tuple,
                [b'}', ..] => Delimiter::MapLike,
                [b',', ..] => Delimiter::Comma,
                [b':', ..] => Delimiter::Colon,
                [b'=', b'>', ..] => Delimiter::FatArrow,
                [b';', ..] => Delimiter::SemiColon,
                [] => Delimiter::EOF,
                _ => break 'found,
            };
            return Ok(Some(found));
        }
        Ok(None)
    }

    fn consume(&mut self, needle: &[u8]) -> bool {
        if self.rest().starts_with(needle) {
            self.bump(needle.len());
            true
        } else {
            false
        }
    }

    fn consume_expected(&mut self, needle: &[u8], reason: ErrorKind) -> ResultKind {
        self.consume(needle).then_some(()).ok_or(reason)
    }

    fn consume_ticks_peek_initiator(&mut self) -> (usize, Option<u8>) {
        let ticks = self.rest().iter().take_while(|&&byte| byte == b'`').count();
        self.bump(ticks);
        if let Some(initiator) = self.rest().first().copied() {
            if initiator < 0x80 {
                return (ticks, Some(initiator));
            }
        }
        (ticks, None)
    }

    fn peek_ticks_from(&self, offset: usize) -> usize {
        self.rest()[offset..].iter().take_while(|&&byte| byte == b'`').count()
    }

    fn peek_integer_kind(&self) -> IntegerKind {
        match self.rest() {
            [b'-', b'0', b'b', ..] | [b'0', b'b', ..] => IntegerKind::Bin,
            [b'-', b'0', b'o', ..] | [b'0', b'o', ..] => IntegerKind::Oct,
            [b'-', b'0', b'x', ..] | [b'0', b'x', ..] => IntegerKind::Hex,
            _ => IntegerKind::Dec,
        }
    }

    fn peek_number_suffix(&self) -> Option<NumberSuffix> {
        'found: {
            let kind = match self.rest() {
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
                _ => break 'found,
            };
            return Some(kind);
        }
        None
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
                return Err(ErrorKind::InvalidByteEscape);
            }
            _ => return Err(ErrorKind::InvalidByteEscape),
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
                return Err(ErrorKind::InvalidAsciiEscape);
            }
            [b'u', rest @ ..] => {
                'unicode: {
                    let Some(b'{') = rest.first() else {
                        break 'unicode;
                    };
                    let Some(len) = rest[1..].iter().position(|&byte| byte == b'}') else {
                        break 'unicode;
                    };
                    let Ok(codep) = lexical_core::parse_with_options::<u32, NUMBER_FORMAT_HEX_NO_PREFIX>(
                        &rest[1..][..len],
                        &PARSE_INTEGER_OPTS,
                    ) else {
                        break 'unicode;
                    };
                    let Some(ch) = char::from_u32(codep) else {
                        break 'unicode;
                    };
                    return Ok((ch, 1 + 1 + len + 1));
                }
                return Err(ErrorKind::InvalidUnicodeEscape);
            }
            _ => return Err(ErrorKind::InvalidAsciiEscape),
        };
        Ok((ch, 1))
    }

    fn parse_u8_fmt_02_hex(bytes: &[u8; 2]) -> ResultKind<u8> {
        Ok(lexical_core::parse_with_options::<u8, NUMBER_FORMAT_HEX_NO_PREFIX>(
            bytes,
            &PARSE_INTEGER_OPTS,
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

        Err(ErrorKind::InvalidUtf8Sequence)
    }

    fn decode_expected(&self) -> ResultKind<(char, usize)> {
        self.decode_from(0)?.ok_or(ErrorKind::UnexpectedEof)
    }

    fn parse_identifier_or_underscore(&mut self) -> ResultKind<Option<IdentRef<'de>>> {
        let raw_mode = self.consume(b"`");
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
                    return Err(ErrorKind::ExpectedIdentifier);
                }
                break;
            }
            if offset == 0 {
                return Err(ErrorKind::UnexpectedEof);
            }
            break;
        }

        let ident = unsafe { core::str::from_utf8_unchecked(&self.rest()[..offset]) };
        if matches!(ident, "_") {
            if raw_mode {
                Err(ErrorKind::UnexpectedUnderscoreIdentifier)
            } else {
                self.bump(offset);
                Ok(None)
            }
        } else {
            if !raw_mode && matches!(ident, "true" | "false" | "inf" | "NaN") {
                Err(ErrorKind::UnexpectedKeywordAsIdentifier)
            } else {
                self.bump(offset);
                Ok(Some(IdentRef::new_unchecked(ident)))
            }
        }
    }
}

impl<'de> Source<'de> for SliceSource<'de> {}

impl<'de> ParseHelper<'de> for SliceSource<'de> {
    fn position(&self) -> Position {
        todo!()
    }

    fn eat_ws(&mut self) -> ResultKind {
        loop {
            self.eat_ws_pure();
            match self.rest() {
                [b'/', b'/', rest @ ..] => {
                    if let Some(off) = memchr(b'\n', rest) {
                        self.bump(2 + off + 1);
                    } else {
                        self.bump_to_end();
                    }
                }
                [b'/', b'*', rest @ ..] => {
                    self.bump(2);
                    let mut lv = 1usize;
                    while lv > 0 {
                        if let Some(off) = memchr2(b'*', b'/', self.rest()) {
                            self.bump(off);
                        }
                        match rest {
                            [b'/', b'*', ..] => {
                                self.bump(2);
                                lv += 1;
                            }
                            [b'*', b'/', ..] => {
                                self.bump(2);
                                lv -= 1;
                            }
                            [_, ..] => (),
                            [] => return Err(ErrorKind::UnclosedBlockComment),
                        }
                    }
                }
                _ => break,
            }
        }
        self.set_position();
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
            None => Err(ErrorKind::ExpectedDelimiter),
        }
    }

    fn adjacent_to_delim(&mut self) -> ResultKind<bool> {
        Ok(self.seek_delim()?.is_some())
    }
}

impl<'de> ParseToConcr<'de> for SliceSource<'de> {
    fn try_byte(&mut self) -> ResultKind<bool> {
        Ok(self.consume(b"b'"))
    }

    fn begin_char(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected(b"'", ErrorKind::ExpectedCharacter)
    }

    fn begin_string(&mut self) -> ResultKind<StringKind> {
        self.eat_ws()?;
        let kind = match self.consume_ticks_peek_initiator() {
            (0, Some(b'"')) => StringKind::Normal,
            (ticks @ 1.., Some(b'"')) => StringKind::Raw { ticks },
            (ticks @ 1.., Some(b'|')) => StringKind::Paragraph { ticks },
            _ => return Err(ErrorKind::ExpectedString),
        };
        self.bump(1);
        Ok(kind)
    }

    fn begin_bytes(&mut self) -> ResultKind<BytesKind> {
        self.eat_ws()?;
        if !self.consume(b"b") {
            return Err(ErrorKind::ExpectedByteString);
        }
        let kind = if let (ticks, Some(b'"')) = self.consume_ticks_peek_initiator() {
            self.bump(1);
            if ticks == 0 {
                BytesKind::Normal
            } else {
                BytesKind::Raw { ticks }
            }
        } else if self.consume(b"64\"") {
            BytesKind::Base64
        } else if self.consume(b"32\"") {
            BytesKind::Base32
        } else if self.consume(b"16\"") {
            BytesKind::Base16
        } else {
            return Err(ErrorKind::ExpectedByteString);
        };
        Ok(kind)
    }

    fn begin_maybe(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected(b"?", ErrorKind::ExpectedMaybe)
    }

    fn begin_array(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected(b"[", ErrorKind::ExpectedArray)
    }

    fn begin_tuple(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected(b"(", ErrorKind::ExpectedTuple)
    }

    fn begin_map_like(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected(b"{", ErrorKind::ExpectedMapLike)
    }

    fn parse_unit(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.consume_expected(b"(", ErrorKind::ExpectedUnit)?;
        self.eat_ws()?;
        self.consume_expected(b")", ErrorKind::ExpectedUnitEnd)
    }

    // NOTE: The following would not eat leading ws.

    fn parse_bool(&mut self) -> ResultKind<bool> {
        if self.consume(b"true") {
            Ok(true)
        } else if self.consume(b"false") {
            Ok(false)
        } else {
            Err(ErrorKind::ExpectedBoolean)
        }
    }

    fn_parse_integer!(i8, parse_i8, NumberSuffix::Int8, ErrorKind::ExpectedInt8);
    fn_parse_integer!(i16, parse_i16, NumberSuffix::Int16, ErrorKind::ExpectedInt16);
    fn_parse_integer!(i32, parse_i32, NumberSuffix::Int32, ErrorKind::ExpectedInt32);
    fn_parse_integer!(i64, parse_i64, NumberSuffix::Int64, ErrorKind::ExpectedInt64);
    fn_parse_integer!(i128, parse_i128, NumberSuffix::Int128, ErrorKind::ExpectedInt128);
    fn_parse_integer!(u8, parse_u8, NumberSuffix::UInt8, ErrorKind::ExpectedUInt8);
    fn_parse_integer!(u16, parse_u16, NumberSuffix::UInt16, ErrorKind::ExpectedUInt16);
    fn_parse_integer!(u32, parse_u32, NumberSuffix::UInt32, ErrorKind::ExpectedUInt32);
    fn_parse_integer!(u64, parse_u64, NumberSuffix::UInt64, ErrorKind::ExpectedUInt64);
    fn_parse_integer!(u128, parse_u128, NumberSuffix::UInt128, ErrorKind::ExpectedUInt128);
    fn_parse_float!(f32, parse_f32, NumberSuffix::Float32, ErrorKind::ExpectedFloat32);
    fn_parse_float!(f64, parse_f64, NumberSuffix::Float64, ErrorKind::ExpectedFloat64);

    fn parse_byte(&mut self) -> ResultKind<u8> {
        let (byte, len) = if self.consume(b"\\") {
            self.peek_escape_byte_from(0)?
        } else {
            let (ch, len) = self.decode_expected()?;
            if !ch.is_ascii() {
                return Err(ErrorKind::UnexpectedNonAsciiCharacter);
            }
            (ch as u8, len)
        };

        self.bump(len);
        self.consume_expected(b"'", ErrorKind::ExpectedUnquote)?;
        Ok(byte)
    }

    fn parse_char(&mut self) -> ResultKind<char> {
        let (ch, len) = if self.consume(b"\\") {
            self.peek_escape_char_from(0)?
        } else {
            self.decode_expected()?
        };

        self.bump(len);
        self.consume_expected(b"'", ErrorKind::ExpectedUnquote)?;
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
                    self.consume(b" ");
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
                    let (r_ticks, Some(initiator @ (b'|' | b'<' | b'>'))) = self.consume_ticks_peek_initiator() else {
                        return Err(ErrorKind::InvalidParagraphLineInitiator);
                    };
                    if ticks != r_ticks {
                        return Err(ErrorKind::UnbalancedRawTicks);
                    }
                    self.bump(1);
                    line_type = initiator;
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

    fn parse_nominal_path<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<NominalPathRef<'t>>
    where
        'de: 't,
    {
        let _ = scratch;
        match self.parse_identifier_or_underscore()? {
            None => Ok(NominalPathRef::Underscore),
            Some(name_or_parent) => {
                if !self.consume(b"::") {
                    Ok(NominalPathRef::Single { name: name_or_parent })
                } else {
                    Ok(NominalPathRef::Dual {
                        name: self
                            .parse_identifier_or_underscore()?
                            .ok_or(ErrorKind::UnexpectedUnderscoreIdentifier)?,
                        parent: name_or_parent,
                    })
                }
            }
        }
    }

    fn parse_identifier<'t>(&mut self, scratch: &'t mut Vec<u8>) -> ResultKind<IdentRef<'t>>
    where
        'de: 't,
    {
        let _ = scratch;
        self.parse_identifier_or_underscore()?
            .ok_or(ErrorKind::UnexpectedUnderscoreIdentifier)
    }
}

impl<'de> ParseToValue<'de> for SliceSource<'de> {
    fn begin(&mut self) -> ResultKind<Indicator<'de>> {
        todo!()
    }

    fn initiator(&mut self) -> ResultKind<Option<Initiator>> {
        todo!()
    }

    fn parse_number(&mut self, kind: NumberKind) -> ResultKind<Either<Number2, NumberNoSuffix2>> {
        todo!()
    }
}
