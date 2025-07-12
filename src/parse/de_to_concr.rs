use super::*;
use chumsky::prelude::*;
use core::num::NonZeroU8;
use lexical_core::{
    NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions, ParseIntegerOptionsBuilder,
};
use serde::{de::Visitor, Deserialize};

type Err = extra::Err<Error>;

pub fn parse<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T> {
    let mut der = Deserializer::new(s)?;
    let value = T::deserialize(&mut der)?;
    der.finish().and(Ok(value))
}

// pub fn parse_many<'de, T: Deserialize<'de>>(s: &'de str) -> Result<Vec<T>> {
//     let mut der = Deserializer::new(s);
//     let value = T::deserialize(&mut der)?;
//     der.finish()?;
//     Ok(value)
// }

pub struct Deserializer<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Deserializer<'a> {
    #[inline]
    pub fn new(source: &'a str) -> Result<Self> {
        let mut der = Self { source, offset: 0 };
        der.consume_whitespace_comment()?;
        Ok(der)
    }

    #[inline]
    pub fn has_reached_end(&self) -> bool {
        self.rest().is_empty()
    }

    #[inline]
    pub fn finish(&mut self) -> Result<()> {
        self.consume_ws_(";")?;

        if self.has_reached_end() {
            Ok(())
        } else {
            Error::raise(ErrorKind::ExpectedEnd)
        }
    }

    #[inline]
    pub fn finish_one(&mut self) -> Result<bool> {
        if self.consume_ws_(";")? {
            Ok(self.has_reached_end())
        } else if self.has_reached_end() {
            Ok(true)
        } else {
            Error::raise(ErrorKind::ExpectedSemiOrEnd)
        }
    }

    #[inline]
    const fn bump(&mut self, n: usize) -> Option<&[u8]> {
        let delta = match self.source.split_at(self.offset).1.split_at_checked(n) {
            Some((_, eps)) => eps,
            None => return None,
        };
        self.offset += n;
        Some(delta.as_bytes())
    }

    #[inline]
    const fn rest(&self) -> &str {
        self.source.split_at(self.offset).1
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
    fn consume_while(&mut self, mut pred: impl FnMut(&char) -> bool) -> &[u8] {
        self.bump(
            self.rest()
                .char_indices()
                .take_while(|(_off, ch)| pred(ch))
                .last()
                .map(|(off, _ch)| off)
                .unwrap_or(0),
        )
        .unwrap()
    }

    #[inline]
    fn consume_whitespace_comment(&mut self) -> Result<()> {
        fn is_whitespace(ch: &char) -> bool {
            ch.is_whitespace()
        }
        fn is_not_newline(ch: &char) -> bool {
            *ch != '\n'
        }
        fn is_not_slash(ch: &char) -> bool {
            *ch != '/'
        }

        loop {
            self.consume_while(is_whitespace);

            if self.consume("//") {
                self.consume_while(is_not_newline);
            } else if self.consume("/*") {
                let mut depth = 1u8;

                while depth != 0 {
                    self.consume_while(is_not_slash);

                    if let Some(b'*') = self.source.as_bytes().get(self.offset - 1) {
                        self.bump(1);
                        depth -= 1;
                    } else if self.consume("/*") {
                        depth += 1;

                        if depth == u8::MAX {
                            return Error::raise(ErrorKind::DeeplyNestedComment);
                        }
                    }

                    if self.has_reached_end() {
                        return Error::raise(ErrorKind::UnclosedComment);
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
        'outer: {
            if let Some(eps) = self.bump(1) {
                return Ok(match eps[0] {
                    b'\\' => b'\\',
                    b'\"' => b'\"',
                    b'\'' => b'\'',
                    b'0' => b'\0',
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    _ => break 'outer,
                });
            }
        }
        Error::raise(ErrorKind::InvalidEscape)
    }

    #[inline]
    fn __escape_byte(&mut self) -> Result<u8> {
        if self.consume("x") {
            if let Some(eps) = self.bump(2) {
                if eps[0].is_ascii_hexdigit() && eps[1].is_ascii_hexdigit() {
                    return Ok(lexical_core::parse::<u8>(eps).unwrap());
                }
            }
        }
        Error::raise(ErrorKind::InvalidEscape)
    }

    #[inline]
    fn __escape_char(&mut self) -> Result<char> {
        if self.consume("x") {
            if let Some(eps) = self.bump(2) {
                if matches!(eps[0], b'0'..=b'7') && eps[1].is_ascii_hexdigit() {
                    return Ok(lexical_core::parse::<u8>(eps).unwrap().into());
                }
            }
        } else if self.consume("u") {
            let eps = self.consume_while(|ch| *ch != '}');
            let chr = lexical_core::parse::<u32>(&eps[1..]).map_err(Into::<Error>::into)?;
            if self.consume("}") {
                if let Some(chr) = char::from_u32(chr) {
                    return Ok(chr);
                }
            }
        }
        Error::raise(ErrorKind::InvalidEscape)
    }

    #[inline]
    fn escape_byte(&mut self) -> Result<Option<u8>> {
        self.consume("\\")
            .then(|| self.__escape_byte().or_else(|_| self.__escape_common()))
            .transpose()
    }

    #[inline]
    fn escape_char(&mut self) -> Result<Option<char>> {
        self.consume("\\")
            .then(|| self.__escape_char().or_else(|_| self.__escape_common().map(Into::into)))
            .transpose()
    }
}

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

const PARSE_INTEGER_OPTS: &ParseIntegerOptions = &ParseIntegerOptionsBuilder::new()
    .no_multi_digit(false)
    .build_unchecked();

const FLOAT_FORMAT: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .required_fraction_digits(false)
    .no_special(false)
    .build();

const PARSE_FLOAT_OPTS: &ParseFloatOptions = &ParseFloatOptionsBuilder::new()
    .lossy(false)
    .nan_string(Some(b"NaN"))
    .inf_string(Some(b"inf"))
    .infinity_string(None)
    .build_unchecked();

macro_rules! deserialize_integer {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        let rest = $self.rest().as_bytes();
        let off = (rest[0] == b'-') as usize;

        let (x, o) = if rest[off] == b'0' && rest[off + 1] == b'x' {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT>(rest, &PARSE_INTEGER_OPTS)
        } else if rest[off] == b'0' && rest[off + 1] == b'o' {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_HEX>(rest, &PARSE_INTEGER_OPTS)
        } else if rest[off] == b'0' && rest[off + 1] == b'b' {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_OCT>(rest, &PARSE_INTEGER_OPTS)
        } else {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_BIN>(rest, &PARSE_INTEGER_OPTS)
        }
        .map_err(Into::<Error>::into)?;

        $self.bump(o);
        $visitor.$method(x)
    }};
}

macro_rules! deserialize_float {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        let (x, o) =
            lexical_core::parse_partial_with_options::<$ty, FLOAT_FORMAT>($self.rest().as_bytes(), &PARSE_FLOAT_OPTS)
                .map_err(Into::<Error>::into)?;
        $self.bump(o);
        $visitor.$method(x)
    }};
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    #![allow(unused_variables)]
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        Error::raise(ErrorKind::WontImplement)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if self.consume_ws_("true")? {
            vis.visit_bool(true)
        } else if self.consume_ws_("false")? {
            vis.visit_bool(false)
        } else {
            Error::raise(ErrorKind::ExpectedBoolean)
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_i64(vis)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_i64(vis)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_i64(vis)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, i64, vis, visit_i64)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, i128, vis, visit_i128)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if self.consume("b'") {
            todo!()
        } else {
            self.deserialize_u64(vis)
        }
    }
    fn deserialize_u16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_u64(vis)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_u64(vis)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, u64, vis, visit_u64)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, u128, vis, visit_u128)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_float!(self, f32, vis, visit_f32)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_float!(self, f64, vis, visit_f64)
    }

    fn deserialize_char<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        'outer: {
            if !self.consume("'") {
                break 'outer;
            }
            let ch = match self.escape_char()? {
                Some(ch) => ch,
                None => match self.rest().chars().next() {
                    Some(ch) => ch,
                    None => break 'outer,
                },
            };
            if !self.consume("'") {
                break 'outer;
            }

            return vis.visit_char(ch);
        }

        Error::raise(ErrorKind::ExpectedCharacter)
    }

    fn deserialize_str<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_string<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_option<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_unit<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_seq<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, len: usize, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_map<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }
}
