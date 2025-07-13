use super::*;
use core::num::NonZeroU8;
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use lexical_core::{
    NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions, ParseIntegerOptionsBuilder,
};
use serde::{
    de::{DeserializeSeed, SeqAccess, Visitor},
    Deserialize,
};

pub fn parse<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T> {
    let mut der = Deserializer::new(s)?;
    let value = T::deserialize(&mut der)?;
    der.finish().and(Ok(value))
}

pub fn parse_many<'de, T: Deserialize<'de>>(s: &'de str) -> Result<Vec<T>> {
    let mut der = Deserializer::new(s)?;
    let mut values = Vec::new();
    loop {
        values.push(T::deserialize(&mut der)?);
        if !der.finish_and_maybe_more()? {
            break;
        }
    }

    Ok(values)
}

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
            self.raise(ErrorKind::ExpectedEnd)
        }
    }

    #[inline]
    pub fn finish_and_maybe_more(&mut self) -> Result<bool> {
        // return: more?
        if self.consume_ws_(";")? {
            Ok(!self.has_reached_end())
        } else if self.has_reached_end() {
            Ok(false)
        } else {
            self.raise(ErrorKind::ExpectedSemiOrEnd)
        }
    }

    #[inline]
    const fn rest(&self) -> &str {
        self.source.split_at(self.offset).1
    }
    #[inline]
    const fn rest_bytes(&self) -> &[u8] {
        self.rest().as_bytes()
    }
    #[inline]
    const fn peek_byte(&self) -> Option<u8> {
        self.rest_bytes().first().copied()
    }
    #[inline]
    const fn adjacent_to_delim(&self) -> bool {
        match self.rest_bytes() {
            [b'=', b'>', ..] | [b')', ..] | [b']', ..] | [b'}', ..] | [b',', ..] | [b';', ..] | [] => true,
            _ => false,
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
    const fn raise<T>(&self, kind: ErrorKind) -> Result<T> {
        self.raise_at(self.offset, kind)
    }
    #[inline]
    const fn raise_at<T>(&self, offset: usize, kind: ErrorKind) -> Result<T> {
        Error::raise_at(offset, kind)
    }
    #[inline]
    const fn raise_unexp_end<T>(&self) -> Result<T> {
        Error::raise_at(self.source.len(), ErrorKind::UnexpectedEnd)
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

        loop {
            self.consume_while(is_whitespace);

            if self.consume("//") {
                self.consume_while(is_not_newline);
            } else if self.consume("/*") {
                let mut depth = 1u8;

                while depth != 0 {
                    let Some(off) = memchr::memchr(b'/', self.rest_bytes()) else {
                        return self.raise_at(self.source.len(), ErrorKind::UnclosedComment);
                    };

                    if let Some(b'*') = self.bump(off).unwrap().last() {
                        self.bump(1);
                        depth -= 1;
                    } else if self.consume("/*") {
                        depth += 1;

                        if depth == u8::MAX {
                            return self.raise(ErrorKind::DeeplyNestedComment);
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
        'outer: {
            if let Some(byte) = self.peek_byte() {
                let byte = match byte {
                    b'\\' => b'\\',
                    b'\"' => b'\"',
                    b'\'' => b'\'',
                    b'0' => b'\0',
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    _ => break 'outer,
                };

                self.bump(1);

                return Ok(byte);
            }
        }
        self.raise(ErrorKind::InvalidEscape)
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
        self.raise(ErrorKind::InvalidEscape)
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
            let off = self.offset;
            let eps = self.consume_while(|ch| *ch != '}');
            let Some((b"{", eps)) = eps.split_at_checked(1) else {
                return self.raise_at(off, ErrorKind::ExpectedSymbol(b'{'));
            };
            let chr = lexical_core::parse_with_options::<
                u32,
                {
                    NumberFormatBuilder::rebuild(lexical_core::format::RUST_LITERAL)
                        .mantissa_radix(16)
                        .build()
                },
            >(eps, &PARSE_INTEGER_OPTS)
            .or_else(|e| self.raise_at(off + 1, e.into()))?;

            return if self.consume("}") {
                if let Some(chr) = char::from_u32(chr) {
                    Ok(chr)
                } else {
                    self.raise_at(off + 1, ErrorKind::InvalidCharacter)
                }
            } else {
                self.raise(ErrorKind::ExpectedSymbol(b'}'))
            };
        }
        self.raise(ErrorKind::InvalidEscape)
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

macro_rules! deserialize_integer {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        let rest = $self.rest_bytes();
        let off = matches!(rest.first(), Some(b'-')) as usize;

        let (x, o) = if matches!(rest.get(off), Some(b'0')) && matches!(rest.get(off + 1), Some(b'x')) {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT>(rest, &PARSE_INTEGER_OPTS)
        } else if matches!(rest.get(off), Some(b'0')) && matches!(rest.get(off + 1), Some(b'o')) {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_HEX>(rest, &PARSE_INTEGER_OPTS)
        } else if matches!(rest.get(off), Some(b'0')) && matches!(rest.get(off + 1), Some(b'b')) {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_OCT>(rest, &PARSE_INTEGER_OPTS)
        } else {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_BIN>(rest, &PARSE_INTEGER_OPTS)
        }
        .or_else(|e| $self.raise(e.into()))?;

        $self.bump(o);
        $visitor.$method(x)
    }};
}

macro_rules! deserialize_float {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        let (x, o) =
            lexical_core::parse_partial_with_options::<$ty, FLOAT_FORMAT>($self.rest_bytes(), &PARSE_FLOAT_OPTS)
                .or_else(|e| $self.raise(e.into()))?;
        $self.bump(o);
        $visitor.$method(x)
    }};
}

macro_rules! maybe_deserialize_baseXX {
    ( $self:ident, $indicator:literal, $decoder:ident, $visitor:ident ) => {{
        if $self.consume($indicator) {
            let Some(off) = ::memchr::memchr(b'"', $self.rest_bytes()) else {
                return $self.raise_unexp_end();
            };

            let buf = $decoder.decode(&$self.rest_bytes()[..off]).map_err(|e| {
                let mut e: Error = e.into();
                e.pos += $self.offset;
                e
            })?;

            $self.bump(off + 1);

            return $visitor.visit_byte_buf(buf);
        }
    }};
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    #![allow(unused_variables)]
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.raise(ErrorKind::WontImplement)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.raise(ErrorKind::WontImplement)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if self.consume_ws_("true")? {
            vis.visit_bool(true)
        } else if self.consume_ws_("false")? {
            vis.visit_bool(false)
        } else {
            self.raise(ErrorKind::ExpectedBoolean)
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, i8, vis, visit_i8)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, i16, vis, visit_i16)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, i32, vis, visit_i32)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, i64, vis, visit_i64)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, i128, vis, visit_i128)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if self.consume("b'") {
            'outer: {
                let byte = match self.escape_byte()? {
                    Some(byte) => byte,
                    None => match self.rest().chars().next() {
                        None => break 'outer,
                        Some(ch) => match ch.try_into() {
                            Ok(byte) => byte,
                            Err(_) => break 'outer,
                        },
                    },
                };

                if !self.consume("'") {
                    return self.raise(ErrorKind::ExpectedSymbol(b'\''));
                }

                return vis.visit_u8(byte);
            }

            self.raise(ErrorKind::ExpectedByteInteger)
        } else {
            deserialize_integer!(self, u8, vis, visit_u8)
        }
    }
    fn deserialize_u16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, u16, vis, visit_u16)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, u32, vis, visit_u32)
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
                return self.raise(ErrorKind::ExpectedSymbol(b'\''));
            }

            return vis.visit_char(ch);
        }

        self.raise(ErrorKind::ExpectedCharacter)
    }

    fn deserialize_string<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_str(vis)
    }
    fn deserialize_str<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_bytes(vis)
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        'outer: {
            let outer_start = self.offset;
            let check_pure_ascii = |s: &str| -> Result<()> {
                s.chars()
                    .all(|ch| ch.is_ascii())
                    .then_some(())
                    .ok_or_else(|| Error::new_at(outer_start, ErrorKind::NonAsciiByteString))
            };

            if !self.consume("b") {
                break 'outer;
            }

            maybe_deserialize_baseXX!(self, "64\"", BASE64URL_NOPAD, vis);
            maybe_deserialize_baseXX!(self, "32\"", BASE32_NOPAD, vis);
            maybe_deserialize_baseXX!(self, "16\"", HEXUPPER_PERMISSIVE, vis);

            let enclosure = self.consume_while(|ch| *ch == '`').len();
            if enclosure >= u8::MAX as _ {
                return self.raise(ErrorKind::ThickRawEnclosure);
            }

            if !self.consume("\"") {
                break 'outer;
            }

            let inner_start = self.offset;
            if enclosure > 0 {
                /* raw bytes */
                loop {
                    let Some(off) = memchr::memchr(b'"', self.rest_bytes()) else {
                        return self.raise_unexp_end();
                    };

                    self.bump(off + 1);

                    if self
                        .rest_bytes()
                        .get(..enclosure)
                        .map(|r| r.iter().all(|byte| *byte == b'`'))
                        .unwrap_or(false)
                    {
                        let bytes = &self.source[inner_start..self.offset - 1];
                        check_pure_ascii(bytes)?;

                        self.bump(enclosure);

                        return vis.visit_borrowed_bytes(bytes.as_bytes());
                    }
                }
            } else {
                /* normal bytes */
                let mut buf = Vec::new();
                let mut cursor = self.offset;

                loop {
                    let Some(off) = memchr::memchr3(b'\\', b'\"', b'\n', self.rest_bytes()) else {
                        return self.raise_unexp_end();
                    };

                    self.bump(off);

                    match self.peek_byte().unwrap() {
                        b'\\' => {
                            let bytes = &self.source[cursor..self.offset];
                            check_pure_ascii(bytes)?;

                            let byte = self.escape_byte()?.unwrap();
                            cursor = self.offset;

                            buf.extend_from_slice(bytes.as_bytes());
                            buf.push(byte);
                        }
                        b'\"' => {
                            self.bump(1);
                            break;
                        }
                        b'\n' => return self.raise(ErrorKind::LinebreakNormalString),

                        _ => unreachable!(),
                    }
                }

                if buf.is_empty() {
                    let bytes = &self.source[inner_start..self.offset - 1];
                    check_pure_ascii(bytes)?;

                    return vis.visit_borrowed_bytes(bytes.as_bytes());
                }

                return vis.visit_byte_buf(buf);
            }
        }

        self.raise(ErrorKind::ExpectedByteString)
    }

    fn deserialize_option<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.consume_ws_("?")?;
        if self.adjacent_to_delim() {
            vis.visit_none()
        } else {
            vis.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if self.consume_ws_("(")? && self.consume_ws_(")")? {
            return vis.visit_unit();
        }

        self.raise_at(start, ErrorKind::ExpectedUnit)
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if self.consume_ws_("_")? || self.consume_ws_("(")? && self.consume_ws_(name)? && self.consume_ws_(")")? {
            return vis.visit_unit();
        }

        self.raise_at(start, ErrorKind::ExpectedUnitStruct(name))
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if (self.consume_ws_("_")? || self.consume_ws_("(")? && self.consume_ws_(name)? && self.consume_ws_(")")?)
            && self.consume_ws_("(")?
        {
            let val = vis.visit_newtype_struct(&mut *self)?;
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::ExpectedSymbol(b')'));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedNewtypeStruct(name))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, len: usize, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if (self.consume_ws_("_")? || self.consume_ws_("(")? && self.consume_ws_(name)? && self.consume_ws_(")")?)
            && self.consume_ws_("(")?
        {
            let val = vis.visit_seq(&mut *self)?;
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::ExpectedSymbol(b')'));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedTupleStruct(name))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        todo!()
    }

    //------------------------------------------------------------------------------

    fn deserialize_seq<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_map<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    //------------------------------------------------------------------------------

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
}

impl<'de> SeqAccess<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.adjacent_to_delim() {
            Ok(None)
        } else {
            let val = seed.deserialize(&mut **self)?;
            self.consume_ws_(",")?;
            Ok(Some(val))
        }
    }
}
