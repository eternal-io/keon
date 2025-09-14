use super::*;
use core::{marker::PhantomData, num::NonZeroU8};
use lexical_core::{
    NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions, ParseIntegerOptionsBuilder,
};
use serde::Deserialize;

pub mod de_to_concr;
pub mod de_to_value;

pub fn parse<'de, T>(s: &'de str) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut der = Parser::new(s)?;
    let value = T::deserialize(&mut der)?;
    der.finish().and(Ok(value))
}

pub fn parse_many<'de, T>(s: &'de str) -> ParseMany<'de, T>
where
    T: Deserialize<'de>,
{
    ParseMany(ParseManyInner::New { src: s })
}

pub struct ParseMany<'de, T>(ParseManyInner<'de, T>);

enum ParseManyInner<'de, T> {
    New { src: &'de str },
    Run { der: Parser<'de>, phantom: PhantomData<T> },
    Exhausted,
}

impl<'de, T> Iterator for ParseMany<'de, T>
where
    T: Deserialize<'de>,
{
    type Item = Result<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let e = 'fail: {
            if let Self(ParseManyInner::Exhausted) = self {
                return None;
            }

            if let Self(ParseManyInner::New { src }) = self {
                match Parser::new(src) {
                    Err(e) => break 'fail e,
                    Ok(der) => {
                        *self = Self(ParseManyInner::Run {
                            der,
                            phantom: PhantomData,
                        })
                    }
                }
            }

            let Self(ParseManyInner::Run { der, .. }) = self else {
                unreachable!()
            };

            let v = match T::deserialize(&mut *der) {
                Err(e) => break 'fail e,
                Ok(v) => v,
            };

            match der.finish_one() {
                Err(e) => break 'fail e,
                Ok(false) => *self = Self(ParseManyInner::Exhausted),
                Ok(true) => (),
            }

            return Some(Ok(v));
        };

        *self = Self(ParseManyInner::Exhausted);

        Some(Err(e))
    }
}

//------------------------------------------------------------------------------

const KEYWORDS: &[&str] = &["NaN", "false", "inf", "long", "true"];

const INTEGER_FORMAT: u128 = lexical_core::format::RUST_LITERAL;

const INTEGER_FORMAT_HEX: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .base_prefix(Some(NonZeroU8::new(b'x').unwrap()))
    .mantissa_radix(16)
    .build();
const INTEGER_FORMAT_OCT: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .base_prefix(Some(NonZeroU8::new(b'o').unwrap()))
    .mantissa_radix(8)
    .build();
const INTEGER_FORMAT_BIN: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .base_prefix(Some(NonZeroU8::new(b'b').unwrap()))
    .mantissa_radix(2)
    .build();

const PARSE_INTEGER_OPTS: ParseIntegerOptions = ParseIntegerOptionsBuilder::new()
    .no_multi_digit(false)
    .build_unchecked();

const FLOAT_FORMAT: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .required_fraction_digits(false)
    .no_special(false)
    .build();

const PARSE_FLOAT_OPTS: ParseFloatOptions = ParseFloatOptionsBuilder::new()
    .lossy(false)
    .nan_string(Some(b"NaN"))
    .inf_string(Some(b"inf"))
    .infinity_string(None)
    .build_unchecked();

fn is_whitespace(ch: &char) -> bool {
    ch.is_whitespace()
}

fn is_backtick(byte: &u8) -> bool {
    *byte == b'`'
}

#[doc(alias = "Deserializer")]
pub struct Parser<'de> {
    src: &'de str,
    pos: usize,
}

impl<'de> Parser<'de> {
    #[inline]
    pub fn new(src: &'de str) -> Result<Self> {
        let mut der = Self { src, pos: 0 };
        der.consume_whitespace_comment()?;
        Ok(der)
    }

    #[inline]
    pub fn has_reached_end(&self) -> bool {
        self.rest().is_empty()
    }

    /// Returns `Ok(_)` if the current value is finished correctly and no more values.
    #[inline]
    pub fn finish(&mut self) -> Result<()> {
        self.consume_ws_(";")?;

        if self.has_reached_end() {
            Ok(())
        } else {
            self.raise(ErrorKind::ExpectedEnd)
        }
    }

    /// Returns `Ok(_)` if the current value is finished correctly.
    /// The `bool` inside indicates whether there are more values.
    #[inline]
    pub fn finish_one(&mut self) -> Result<bool> {
        if self.consume_ws_(";")? {
            Ok(!self.has_reached_end())
        } else if self.has_reached_end() {
            Ok(false)
        } else {
            self.raise(ErrorKind::ExpectedSemiOrEnd)
        }
    }

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
    const fn raise<T>(&self, kind: ErrorKind) -> Result<T> {
        self.raise_at(self.pos, kind)
    }
    #[inline]
    const fn raise_at<T>(&self, pos: usize, kind: ErrorKind) -> Result<T> {
        Error::raise_at(pos, kind)
    }
    #[inline]
    const fn raise_unexpected_end<T>(&self) -> Result<T> {
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
        Ok(match self.rest().starts_with(pat) {
            false => false,
            true => {
                self.bump(pat.len());
                self.consume_whitespace_comment()?;
                true
            }
        })
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

    #[inline]
    fn consume_ident(&mut self) -> Result<&'de str> {
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
            return self.raise_at(start, ErrorKind::UnderscoreIdent);
        }

        let end = self.pos;
        self.consume_whitespace_comment()?;

        let ident = &self.src[start..end];
        if !raw_mode {
            if let Some(keyword) = KEYWORDS.iter().find(|kw| **kw == ident) {
                return self.raise_at(start, ErrorKind::UnexpectedKeyword { keyword });
            }
        }

        Ok(ident)
    }

    #[inline]
    fn consume_ident_exact(&mut self, ident: &'static str) -> Result<bool> {
        let start = self.pos;
        if !self.consume("`") {
            if let Some(keyword) = KEYWORDS.iter().find(|kw| **kw == ident) {
                return self.raise_at(start, ErrorKind::UnexpectedKeyword { keyword });
            }
        }

        if self.consume_ws_(ident)? {
            Ok(true)
        } else {
            self.pos = start;
            Ok(false)
        }
    }

    #[inline]
    fn consume_newline(&mut self) -> Result<()> {
        let start = self.pos;
        self.consume_while_fast(|byte| *byte == b'\r');
        if !self.consume("\n") {
            self.raise_at(start, ErrorKind::UnexpectedCarriageReturn)
        } else {
            Ok(())
        }
    }

    #[inline]
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
                    return self.raise_at(start, ErrorKind::Expected("`{`"));
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
                    self.raise(ErrorKind::Expected("`}`"))
                }
            })
        }
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
}
