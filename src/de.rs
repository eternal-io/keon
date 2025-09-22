use self::error::*;
use crate::value::*;
use core::{cmp::Ordering, marker::PhantomData, num::NonZeroU8, ops::Neg};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use lexical_core::{
    FromLexicalWithOptions, NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions,
    ParseIntegerOptionsBuilder,
};

pub mod de_to_concr;
pub mod de_to_value;
pub mod error;

pub fn parse<'de, T: Parsable<'de>>(s: &'de str) -> Result<T> {
    let mut der = Parser::new(s);
    let val = der.parse_one()?;
    der.finish().and(Ok(val))
}

pub fn parse_limited<'de, T: Parsable<'de>>(s: &'de str, limit: Option<u32>) -> Result<T> {
    let mut der = Parser::new(s);
    let val = der.parse_one_limited(limit)?;
    der.finish().and(Ok(val))
}

pub fn parse_many<'de, T: Parsable<'de>>(s: &'de str) -> IterParser<'de, T> {
    Parser::new(s).into_iter()
}

pub fn parse_many_limited<'de, T: Parsable<'de>>(s: &'de str, limit: Option<u32>) -> IterParser<'de, T> {
    Parser::new(s).into_limited_iter(limit)
}

//------------------------------------------------------------------------------

#[doc(alias = "Deserialize")]
pub trait Parsable<'de>: Sized {
    #[inline]
    fn parse_via(der: &mut Parser<'de>) -> Result<Self> {
        Self::parse_limited_via(der, None)
    }

    fn parse_limited_via(der: &mut Parser<'de>, limit: Option<u32>) -> Result<Self>;
}

#[doc(alias = "Deserializer")]
pub struct Parser<'de> {
    src: &'de str,
    pos: usize,

    /// To avoid possible failures when creating a parser,
    /// whitespace handling is moved to the first value being parsed.
    /// However, only the first value requires additional whitespace handling,
    /// as [`Self::consume_ws_`] handles all leading whitespace after the first value.
    /// Therefore, this flag exists to allow an important optimization.
    leading_ws_handled: bool,

    /// If a parser has previously failed, to prevent it from being used again,
    /// all subsequent uses of failable methods will fail with [`ErrorKind::Corrupted`].
    ///
    /// This flag is primarily maintained by `raise_` methods. Implementations should not
    /// forget to set this flag if the [`Result`] is not constructed via a parser (e.g.
    /// via [`serde::de::Visitor`]). Besides, all non-trait methods on this parser are
    /// guaranteed to handle this flag correctly.
    corrupted: bool,
}

impl<'de> Parser<'de> {
    #[inline]
    pub fn new(src: &'de str) -> Self {
        Self {
            src,
            pos: 0,
            leading_ws_handled: false,
            corrupted: false,
        }
    }

    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter<T: Parsable<'de>>(self) -> IterParser<'de, T> {
        IterParser {
            der: self,
            ttl: None,
            typ: PhantomData,
        }
    }

    #[inline]
    pub fn into_limited_iter<T: Parsable<'de>>(self, limit: Option<u32>) -> IterParser<'de, T> {
        IterParser {
            der: self,
            ttl: limit,
            typ: PhantomData,
        }
    }

    #[inline]
    pub fn parse_one<T: Parsable<'de>>(&mut self) -> Result<T> {
        T::parse_via(self)
    }

    #[inline]
    pub fn parse_one_limited<T: Parsable<'de>>(&mut self, limit: Option<u32>) -> Result<T> {
        T::parse_limited_via(self, limit)
    }

    /// Returns `Ok(_)` if the current value is finished correctly and no more values.
    #[inline]
    pub fn finish(&mut self) -> Result<()> {
        if self.corrupted {
            self.raise(ErrorKind::Corrupted)
        } else {
            self.consume_ws_(";")?;
            if !self.has_reached_end() {
                self.raise(ErrorKind::ExpectedEnd)
            } else {
                Ok(())
            }
        }
    }

    /// Returns `Ok(_)` if the current value is finished correctly.
    /// The `bool` inside indicates whether there are more values.
    ///
    /// Call this method before deserializing every next value.
    #[inline]
    pub fn finish_one(&mut self) -> Result<bool> {
        if self.corrupted {
            self.raise(ErrorKind::Corrupted)
        } else if self.consume_ws_(";")? {
            Ok(!self.has_reached_end())
        } else if self.has_reached_end() {
            Ok(false)
        } else {
            self.raise(ErrorKind::ExpectedSemiOrEnd)
        }
    }

    #[inline]
    pub fn has_reached_end(&self) -> bool {
        self.rest().is_empty()
    }

    #[inline]
    pub fn is_corrupted(&self) -> bool {
        self.corrupted
    }
}

//------------------------------------------------------------------------------

/// NOTE:
/// As an iterator, once the internal parser becomes corrupted,
/// it will always return `Some(Err(_))` with [`ErrorKind::Corrupted`] .
pub struct IterParser<'de, T> {
    der: Parser<'de>,
    ttl: Option<u32>,
    typ: PhantomData<T>,
}

impl<'de, T> Iterator for IterParser<'de, T>
where
    T: Parsable<'de>,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.der.is_corrupted() {
            return Some(self.der.raise(ErrorKind::Corrupted));
        }
        if self.der.has_reached_end() {
            return None;
        }

        let e = 'fail: {
            let v = match self.der.parse_one_limited(self.ttl) {
                Ok(v) => v,
                Err(e) => break 'fail e,
            };
            if let Err(e) = self.der.finish_one() {
                break 'fail e;
            }

            return Some(Ok(v));
        };

        Some(Err(e))
    }
}

impl<'de, T> IterParser<'de, T> {
    #[inline]
    pub fn into_inner(self) -> Parser<'de> {
        self.der
    }

    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.der.has_reached_end()
    }

    #[inline]
    pub fn is_corrupted(&self) -> bool {
        self.der.is_corrupted()
    }

    #[inline]
    pub fn set_limit(&mut self, limit: Option<u32>) {
        self.ttl = limit;
    }
}

//------------------------------------------------------------------------------

fn is_whitespace(ch: &char) -> bool {
    ch.is_whitespace()
}

fn is_backtick(byte: &u8) -> bool {
    *byte == b'`'
}

enum Token<'de> {
    Keyword(Keyword),
    Identifier(&'de str),
    Underscore,
}

enum Keyword {
    NotANumber,
    False,
    Infinity,
    Long,
    True,
}

/// The keyword list is sorted and must be sorted.
const KEYWORDS: &[&str] = &["NaN", "false", "inf", "long", "true"];

impl From<Keyword> for &'static str {
    fn from(value: Keyword) -> Self {
        KEYWORDS[value as usize]
    }
}

impl TryFrom<&str> for Keyword {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        KEYWORDS.binary_search(&s).or(Err(())).map(|idx| match idx {
            0 => Self::NotANumber,
            1 => Self::False,
            2 => Self::Infinity,
            3 => Self::Long,
            4 => Self::True,
            _ => unreachable!(),
        })
    }
}

impl<'de> Parser<'de> {
    #[inline]
    const fn rest(&self) -> &'de str {
        self.src.split_at(self.pos).1
    }
    #[inline]
    const fn rest_bytes(&self) -> &'de [u8] {
        self.source_bytes().split_at(self.pos).1
    }
    #[inline]
    const fn source_bytes(&self) -> &'de [u8] {
        self.src.as_bytes()
    }
    #[inline]
    const fn peek_byte(&self) -> Option<u8> {
        self.rest_bytes().first().copied()
    }
    #[inline]
    const fn adjacent_to_delim(&self) -> bool {
        matches!(
            self.rest_bytes(),
            [b'=', b'>', ..] | [b')', ..] | [b']', ..] | [b'}', ..] | [b',', ..] | [b';', ..] | []
        )
    }

    #[inline]
    const fn bump(&mut self, n: usize) -> &'de str {
        let delta = self.rest().split_at(n).0;
        self.pos += n;
        delta
    }
    #[inline]
    const fn bump_to_end(&mut self) -> &'de str {
        let delta = self.rest();
        self.pos = self.src.len();
        delta
    }
    #[inline]
    const fn try_bump(&mut self, n: usize) -> Option<&'de [u8]> {
        if self.src.is_char_boundary(self.pos + n) {
            let delta = self.rest_bytes().split_at(n).0;
            self.pos += n;
            Some(delta)
        } else {
            None
        }
    }

    #[inline]
    fn raise<T>(&mut self, kind: ErrorKind) -> Result<T> {
        self.raise_at(self.pos, kind)
    }
    #[inline]
    fn raise_at<T>(&mut self, pos: usize, kind: ErrorKind) -> Result<T> {
        self.corrupted = true;
        Error::raise_at(pos, kind)
    }
    #[inline]
    fn raise_unexpected_end<T>(&mut self) -> Result<T> {
        self.corrupted = true;
        Error::raise_at(self.src.len(), ErrorKind::UnexpectedEnd)
    }

    #[inline]
    fn consume(&mut self, pat: &'static str) -> bool {
        match self.rest().starts_with(pat) {
            false => false,
            true => {
                self.bump(pat.len());
                true
            }
        }
    }

    #[inline]
    fn consume_if(&mut self, pred: impl FnOnce(&char) -> bool) -> bool {
        match self.rest().chars().next().filter(pred) {
            None => false,
            Some(ch) => {
                self.bump(ch.len_utf8());
                true
            }
        }
    }

    #[inline]
    fn consume_ws_(&mut self, pat: &'static str) -> Result<bool> {
        match self.rest().starts_with(pat) {
            false => Ok(false),
            true => {
                self.bump(pat.len());
                self.consume_whitespace_comment()?;
                Ok(true)
            }
        }
    }

    #[inline]
    fn consume_while(&mut self, mut pred: impl FnMut(&char) -> bool) -> &'de [u8] {
        self.bump(
            self.rest()
                .char_indices()
                .take_while(|(_off, ch)| pred(ch))
                .last()
                .map(|(off, ch)| off + ch.len_utf8())
                .unwrap_or(0),
        )
        .as_bytes()
    }

    #[inline]
    fn consume_while_fast(&mut self, mut pred: impl FnMut(&u8) -> bool) -> &'de [u8] {
        self.bump(
            self.rest_bytes()
                .iter()
                .enumerate()
                .take_while(|(_off, byte)| pred(byte))
                .last()
                .map(|(off, _byte)| off + 1)
                .unwrap_or(0),
        )
        .as_bytes()
    }

    fn consume_ident(&mut self) -> Result<&'de str> {
        let start = self.pos;
        match self.consume_keyword_or_ident_or_underscore()? {
            Token::Keyword(kw) => self.raise_at(start, ErrorKind::UnexpectedKeywordIdent(kw.into())),

            Token::Identifier(ident) => Ok(ident),

            Token::Underscore => self.raise_at(start, ErrorKind::UnexpectedUnderscoreIdent),
        }
    }

    fn consume_ident_or_underscore(&mut self) -> Result<Option<&'de str>> {
        let start = self.pos;
        match self.consume_keyword_or_ident_or_underscore()? {
            Token::Keyword(kw) => self.raise_at(start, ErrorKind::UnexpectedKeywordIdent(kw.into())),

            Token::Identifier(ident) => Ok(Some(ident)),

            Token::Underscore => Ok(None),
        }
    }

    fn consume_keyword_or_ident_or_underscore(&mut self) -> Result<Token<'de>> {
        let raw_mode = self.consume("`");
        let start = self.pos;
        let need_more = if self.consume("_") {
            true
        } else if self.consume_if(|ch| unicode_ident::is_xid_start(*ch)) {
            false
        } else {
            return self.raise_at(start, ErrorKind::ExpectedIdent);
        };

        let no_more = self.consume_while(|ch| unicode_ident::is_xid_continue(*ch)).is_empty();
        if need_more && no_more {
            return Ok(Token::Underscore);
        }

        let end = self.pos;

        self.consume_whitespace_comment()?;

        let ident = &self.src[start..end];
        let token = if !raw_mode {
            if let Ok(kw) = Keyword::try_from(ident) {
                Token::Keyword(kw)
            } else {
                Token::Identifier(ident)
            }
        } else {
            Token::Identifier(ident)
        };

        Ok(token)
    }

    #[inline]
    fn consume_newline(&mut self) -> Result<()> {
        self.consume("\r");
        if !self.consume("\n") {
            self.raise(ErrorKind::UnexpectedCarriageReturn)
        } else {
            Ok(())
        }
    }

    fn consume_whitespace_comment(&mut self) -> Result<()> {
        loop {
            self.consume_while(is_whitespace);

            if self.consume("//") {
                match memchr::memchr(b'\n', self.rest_bytes()) {
                    Some(off) => {
                        self.bump(off);
                    }
                    None => {
                        self.pos = self.src.len();
                        break;
                    }
                }
            } else if self.consume("/*") {
                let mut depth = 1u8;

                while depth != 0 {
                    let Some(off) = memchr::memchr(b'/', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    if let Some(b'*') = self.bump(off).as_bytes().last() {
                        self.bump(1);
                        depth -= 1;
                    } else if self.consume("/*") {
                        depth = match depth.checked_add(1) {
                            Some(n) => n,
                            None => return self.raise(ErrorKind::DeeplyNestedComment),
                        }
                    }
                }
            } else {
                break;
            }
        }

        self.consume_while(is_whitespace);

        Ok(())
    }

    fn consume_whitespace_comment_first(&mut self) -> Result<()> {
        if !self.leading_ws_handled {
            self.consume_whitespace_comment()?;
            self.leading_ws_handled = true;
        }

        Ok(())
    }

    #[inline]
    fn escape_byte(&mut self) -> Option<Result<u8>> {
        if self.consume("\\") {
            self.__escape_byte().or_else(|| Some(self.__escape_common()))
        } else {
            None
        }
    }

    #[inline]
    fn escape_char(&mut self) -> Option<Result<char>> {
        if self.consume("\\") {
            self.__escape_char()
                .or_else(|| Some(self.__escape_common().map(Into::into)))
        } else {
            None
        }
    }

    #[inline]
    fn __escape_common(&mut self) -> Result<u8> {
        if let Some(byte) = self.peek_byte().and_then(|byte| {
            Some(match byte {
                b'\\' => b'\\',
                b'\"' => b'\"',
                b'\'' => b'\'',
                b'0' => b'\0',
                b'n' => b'\n',
                b't' => b'\t',
                b'r' => b'\r',
                _ => return None,
            })
        }) {
            self.bump(1);
            Ok(byte)
        } else {
            self.raise(ErrorKind::InvalidEscape)
        }
    }

    #[inline]
    fn __escape_byte(&mut self) -> Option<Result<u8>> {
        if self.consume("x") {
            Some({
                if let Some(delta) = self.try_bump(2) {
                    if delta[0].is_ascii_hexdigit() && delta[1].is_ascii_hexdigit() {
                        return Some(Ok(lexical_core::parse::<u8>(delta).unwrap()));
                    }
                }
                self.raise(ErrorKind::InvalidByteEscape)
            })
        } else {
            None
        }
    }

    #[inline]
    fn __escape_char(&mut self) -> Option<Result<char>> {
        if self.consume("x") {
            Some({
                if let Some(delta) = self.try_bump(2) {
                    if matches!(delta[0], b'0'..=b'7') && delta[1].is_ascii_hexdigit() {
                        return Some(Ok(lexical_core::parse::<u8>(delta).unwrap().into()));
                    }
                }
                self.raise(ErrorKind::InvalidAsciiEscape)
            })
        } else {
            self.consume("u").then(|| {
                let start = self.pos;
                let delta = self.consume_while(|ch| *ch != '}');
                let Some((b'{', delta)) = delta.split_first() else {
                    return self.raise_at(start, ErrorKind::ExpectedBraceOpen);
                };

                let chr = lexical_core::parse_with_options::<
                    u32,
                    {
                        NumberFormatBuilder::rebuild(lexical_core::format::RUST_LITERAL)
                            .mantissa_radix(16)
                            .build()
                    },
                >(delta, &PARSE_INTEGER_OPTS)
                .or_else(|_| self.raise_at(start + 1, ErrorKind::InvalidUnicodeEscape))?;

                if self.consume("}") {
                    if let Some(chr) = char::from_u32(chr) {
                        Ok(chr)
                    } else {
                        self.raise_at(start + 1, ErrorKind::InvalidUnicodeEscape)
                    }
                } else {
                    self.raise(ErrorKind::ExpectedBraceClose)
                }
            })
        }
    }
}

//------------------------------------------------------------------------------

const NUMBER_FORMAT: u128 = NumberFormatBuilder::new()
    .digit_separator(NonZeroU8::new(b'_'))
    .internal_digit_separator(true)
    .trailing_digit_separator(true)
    .consecutive_digit_separator(true)
    .no_positive_mantissa_sign(true)
    .case_sensitive_special(true)
    .build();

const NUMBER_FORMAT_HEX: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT).mantissa_radix(16).build();
const NUMBER_FORMAT_OCT: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT).mantissa_radix(8).build();
const NUMBER_FORMAT_BIN: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT).mantissa_radix(2).build();

const PARSE_INTEGER_OPTS: ParseIntegerOptions = ParseIntegerOptionsBuilder::new()
    .no_multi_digit(false)
    .build_unchecked();

const PARSE_FLOAT_OPTS: ParseFloatOptions = ParseFloatOptionsBuilder::new()
    .lossy(false)
    .exponent(b'e')
    .decimal_point(b'.')
    .nan_string(Some(b"NaN"))
    .inf_string(Some(b"inf"))
    .infinity_string(None)
    .build_unchecked();

trait ToSigned {
    type Signed;
    fn to_signed(self, neg: bool) -> Result<Self::Signed, ErrorKind>;
}

macro_rules! impl_integer_to_signed {
    ( $ty:ident => $out:ident ) => {
        impl ToSigned for $ty {
            type Signed = $out;
            #[inline]
            fn to_signed(self, neg: bool) -> Result<Self::Signed, ErrorKind> {
                if neg {
                    if self <= $out::MIN.unsigned_abs() {
                        Ok((!self).wrapping_add(1) as $out)
                    } else {
                        Err(ErrorKind::IntegerUnderflow)
                    }
                } else if self > $out::MAX as $ty {
                    Err(ErrorKind::IntegerOverflow)
                } else {
                    Ok(self as $out)
                }
            }
        }
    };
}

impl_integer_to_signed!(u8 => i8);
impl_integer_to_signed!(u16 => i16);
impl_integer_to_signed!(u32 => i32);
impl_integer_to_signed!(u64 => i64);
impl_integer_to_signed!(u128 => i128);

impl<'de> Parser<'de> {
    fn parse_integer_unsigned<T>(&mut self) -> Result<T>
    where
        T: FromLexicalWithOptions<Options = ParseIntegerOptions>,
    {
        let start = self.pos;
        let (num, off) = if self.consume("0x") {
            lexical_core::parse_partial_with_options::<T, NUMBER_FORMAT_HEX>(self.rest_bytes(), &PARSE_INTEGER_OPTS)
        } else if self.consume("0o") {
            lexical_core::parse_partial_with_options::<T, NUMBER_FORMAT_OCT>(self.rest_bytes(), &PARSE_INTEGER_OPTS)
        } else if self.consume("0b") {
            lexical_core::parse_partial_with_options::<T, NUMBER_FORMAT_BIN>(self.rest_bytes(), &PARSE_INTEGER_OPTS)
        } else {
            lexical_core::parse_partial_with_options::<T, NUMBER_FORMAT>(self.rest_bytes(), &PARSE_INTEGER_OPTS)
        }
        .map_err(|e| {
            self.corrupted = true;
            let mut e: Error = e.into();
            e.pos += self.pos;
            e
        })?;

        self.bump(off);

        if self
            .peek_byte()
            .map(|byte| byte.is_ascii_alphanumeric() || byte == b'.')
            .unwrap_or(false)
        {
            return self.raise_at(start, ErrorKind::InvalidNumber);
        }

        self.consume_whitespace_comment()?;

        Ok(num)
    }

    fn parse_integer_signed<T>(&mut self) -> Result<T::Signed>
    where
        T: FromLexicalWithOptions<Options = ParseIntegerOptions> + ToSigned,
    {
        let neg = self.consume_ws_("-")?;
        let start = self.pos;
        let num = self.parse_integer_unsigned::<T>()?;
        let num = num.to_signed(neg).or_else(|kind| self.raise_at(start, kind))?;

        Ok(num)
    }

    fn parse_integer_either_with_known<T>(&mut self, neg: bool) -> Result<Either<T, T::Signed>>
    where
        T: FromLexicalWithOptions<Options = ParseIntegerOptions> + ToSigned,
    {
        let start = self.pos;
        let num = self.parse_integer_unsigned::<T>()?;
        let num = match neg {
            true => Either::Right(num.to_signed(true).or_else(|kind| self.raise_at(start, kind))?),
            false => Either::Left(num),
        };

        Ok(num)
    }

    fn parse_float<T>(&mut self) -> Result<T>
    where
        T: Neg<Output = T> + FromLexicalWithOptions<Options = ParseFloatOptions>,
    {
        let neg = self.consume_ws_("-")?;
        self.parse_float_with_known(neg)
    }

    fn parse_float_with_known<T>(&mut self, neg: bool) -> Result<T>
    where
        T: Neg<Output = T> + FromLexicalWithOptions<Options = ParseFloatOptions>,
    {
        let start = self.pos;
        let (num, off) =
            lexical_core::parse_partial_with_options::<T, NUMBER_FORMAT>(self.rest_bytes(), &PARSE_FLOAT_OPTS)
                .map_err(|e| {
                    self.corrupted = true;
                    let mut e: Error = e.into();
                    e.pos += start;
                    e
                })?;

        self.bump(off);

        if self
            .peek_byte()
            .map(|byte| byte.is_ascii_alphanumeric() || byte == b'.')
            .unwrap_or(false)
        {
            return self.raise_at(start, ErrorKind::InvalidNumber);
        }

        self.consume_whitespace_comment()?;

        Ok(if neg { -num } else { num })
    }
}

//------------------------------------------------------------------------------

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    fn converge<T>(self) -> T
    where
        L: Into<T>,
        R: Into<T>,
    {
        match self {
            Either::Left(left) => left.into(),
            Either::Right(right) => right.into(),
        }
    }
}

macro_rules! maybe_deserialize_baseXX {
    ( $self:ident, $indicator:literal, $decoder:ident ) => {{
        if $self.consume($indicator) {
            let rest = $self.rest_bytes();
            let Some(off) = memchr::memchr(b'"', rest) else {
                return $self.raise_unexpected_end();
            };

            let buf = $decoder.decode(&rest[..off]).map_err(|e| {
                $self.corrupted = true;
                let mut e: Error = e.into();
                e.pos += $self.pos;
                e
            })?;

            $self.bump(off + 1);
            $self.consume_whitespace_comment()?;

            return Ok(Either::Right(buf));
        }
    }};
}

impl<'de> Parser<'de> {
    #[inline]
    fn parse_char(&mut self) -> Result<char> {
        let start = self.pos;

        'char: {
            if !self.consume("'") {
                break 'char;
            }

            let ch = if let Some(ch) = self.escape_char() {
                ch?
            } else if let Some(ch) = self.rest().chars().next() {
                self.bump(ch.len_utf8());
                ch
            } else {
                break 'char;
            };

            if !self.consume_ws_("'")? {
                return self.raise(ErrorKind::ExpectedQuote);
            }

            return Ok(ch);
        }

        self.raise_at(start, ErrorKind::ExpectedCharacter)
    }

    #[inline]
    fn parse_byte(&mut self) -> Result<u8> {
        let start = self.pos;

        'byte: {
            if !self.consume("b'") {
                break 'byte;
            }

            let byte = if let Some(byte) = self.escape_byte() {
                byte?
            } else if let Some(byte) = self.rest().chars().next().filter(|ch| ch.is_ascii()) {
                self.bump(1);
                byte as u8
            } else {
                break 'byte;
            };

            if !self.consume_ws_("'")? {
                return self.raise(ErrorKind::ExpectedQuote);
            }

            return Ok(byte);
        }

        self.raise_at(start, ErrorKind::ExpectedByteInteger)
    }

    #[inline]
    fn parse_byte_string(&mut self) -> Result<Either<&'de [u8], ByteBuf>> {
        #[inline]
        fn filter_non_ascii(s: &str, der: &mut Parser) -> Result<()> {
            match s.as_bytes().iter().enumerate().find(|(_off, byte)| !byte.is_ascii()) {
                Some((off, _byte)) => der.raise_at(der.pos - s.len() + off, ErrorKind::UnexpectedNonAsciiCharacter),
                None => Ok(()),
            }
        }

        let start = self.pos;

        'byte_string: {
            if !self.consume("b") {
                break 'byte_string;
            }

            maybe_deserialize_baseXX!(self, "64\"", BASE64URL_NOPAD);
            maybe_deserialize_baseXX!(self, "32\"", BASE32_NOPAD);
            maybe_deserialize_baseXX!(self, "16\"", HEXUPPER_PERMISSIVE);

            let delim_len = self.consume_while_fast(is_backtick).len();

            if !self.consume("\"") {
                break 'byte_string;
            }

            let mut buf = Vec::new();
            let mut cursor = self.pos; // initialized as `inner_start`.
            let mut inner_end;

            match delim_len {
                0 => loop {
                    /* normal byte string */
                    let Some(off) = memchr::memchr3(b'\"', b'\\', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    filter_non_ascii(self.bump(off), self)?;

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            self.consume_newline()?;
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(b'\n');
                        }
                        b'\\' => {
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(self.escape_byte().unwrap()?);
                        }
                        b'\"' => {
                            inner_end = self.pos;
                            self.bump(1);
                            break;
                        }
                        _ => unreachable!(),
                    }

                    cursor = self.pos;
                },

                _ => loop {
                    /* raw byte string */
                    let Some(off) = memchr::memchr2(b'\"', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    filter_non_ascii(self.bump(off), self)?;

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            self.consume_newline()?;
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(b'\n');
                        }
                        b'\"' => {
                            inner_end = self.pos;
                            self.bump(1);
                            match self.consume_while_fast(is_backtick).len().cmp(&delim_len) {
                                Ordering::Less => (),
                                Ordering::Equal => break,
                                Ordering::Greater => {
                                    return self.raise_at(inner_end + 1, ErrorKind::UnbalancedRawDelimiters)
                                }
                            }
                        }
                        _ => unreachable!(),
                    }

                    cursor = self.pos;
                },
            }

            self.consume_whitespace_comment()?;

            return if buf.is_empty() {
                Ok(Either::Left(&self.source_bytes()[cursor..inner_end]))
            } else {
                buf.extend_from_slice(&self.source_bytes()[cursor..inner_end]);
                Ok(Either::Right(buf))
            };
        }

        self.raise_at(start, ErrorKind::ExpectedByteString)
    }

    #[inline]
    fn parse_string_or_paragraph(&mut self) -> Result<Either<&'de str, String>> {
        let start = self.pos;
        let delim_len = self.consume_while_fast(is_backtick).len();

        if self.consume("\"") {
            self._parse_string(delim_len)
        } else if delim_len > 0 && self.consume("|") {
            self._parse_paragraph(delim_len)
        } else {
            self.raise_at(start, ErrorKind::ExpectedStringOrParagraph)
        }
    }

    #[inline]
    fn _parse_string(&mut self, delim_len: usize) -> Result<Either<&'de str, String>> {
        let mut buf = String::new();
        let mut cursor = self.pos; // initialized as `inner_start`.
        let mut inner_end;

        match delim_len {
            0 => loop {
                /* normal string */
                let Some(off) = memchr::memchr3(b'\"', b'\\', b'\r', self.rest_bytes()) else {
                    return self.raise_unexpected_end();
                };

                self.bump(off);

                match self.peek_byte().unwrap() {
                    b'\r' => {
                        self.consume_newline()?;
                        buf.push_str(&self.src[cursor..self.pos]);
                        buf.push('\n');
                    }
                    b'\\' => {
                        buf.push_str(&self.src[cursor..self.pos]);
                        buf.push(self.escape_char().unwrap()?);
                    }
                    b'\"' => {
                        inner_end = self.pos;
                        self.bump(1);
                        break;
                    }
                    _ => unreachable!(),
                }

                cursor = self.pos;
            },

            _ => loop {
                /* raw string */
                let Some(off) = memchr::memchr2(b'\"', b'\r', self.rest_bytes()) else {
                    return self.raise_unexpected_end();
                };

                self.bump(off);

                match self.peek_byte().unwrap() {
                    b'\r' => {
                        self.consume_newline()?;
                        buf.push_str(&self.src[cursor..self.pos]);
                        buf.push('\n');
                    }
                    b'\"' => {
                        inner_end = self.pos;
                        self.bump(1);
                        match self.consume_while_fast(is_backtick).len().cmp(&delim_len) {
                            Ordering::Less => (),
                            Ordering::Equal => break,
                            Ordering::Greater => {
                                return self.raise_at(inner_end + 1, ErrorKind::UnbalancedRawDelimiters)
                            }
                        }
                    }
                    _ => unreachable!(),
                }

                cursor = self.pos;
            },
        }

        self.consume_whitespace_comment()?;

        if buf.is_empty() {
            Ok(Either::Left(&self.src[cursor..inner_end]))
        } else {
            buf.push_str(&self.src[cursor..inner_end]);
            Ok(Either::Right(buf))
        }
    }

    #[inline]
    fn _parse_paragraph(&mut self, delim_len: usize) -> Result<Either<&'de str, String>> {
        #[inline]
        fn trim(s: &str) -> &str {
            if let Some((b' ', s)) = s.as_bytes().split_first() {
                unsafe { core::str::from_utf8_unchecked(s) }
            } else {
                s
            }
            .trim_end()
        }

        let mut buf = String::new();
        let mut firstline;
        match memchr::memchr2(b'\r', b'\n', self.rest_bytes()) {
            None => firstline = Some(trim(self.bump_to_end())),
            Some(off) => {
                firstline = Some(trim(self.bump(off)));
                self.consume_newline()?;
                self.consume_whitespace_comment()?;
            }
        }

        loop {
            if self.adjacent_to_delim() {
                break;
            }
            if self.consume_while_fast(is_backtick).len() != delim_len {
                return self.raise(ErrorKind::UnbalancedRawDelimiters);
            }
            let Some(sym) = self.peek_byte() else {
                return self.raise(ErrorKind::InvalidParagraphLine);
            };
            if !matches!(sym, b'|' | b'<' | b'>') {
                return self.raise(ErrorKind::InvalidParagraphLine);
            }

            self.bump(1);

            let contiline;
            match memchr::memchr2(b'\r', b'\n', self.rest_bytes()) {
                None => contiline = trim(self.bump_to_end()),
                Some(off) => {
                    contiline = trim(self.bump(off));
                    self.consume_newline()?;
                    self.consume_whitespace_comment()?;
                }
            }

            if let Some(first) = firstline.take() {
                buf.push_str(first);
            }
            if sym == b'|' {
                buf.push('\n');
            }
            if sym == b'>' && !contiline.is_empty() {
                buf.push(' ');
            }

            buf.push_str(contiline);
        }

        if let Some(s) = firstline {
            Ok(Either::Left(s))
        } else {
            Ok(Either::Right(buf))
        }
    }
}
