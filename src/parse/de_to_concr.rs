use super::*;
use core::num::NonZeroU8;
use core::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use lexical_core::{
    NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions, ParseIntegerOptionsBuilder,
};
use serde::{
    de::{value::BorrowedStrDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
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

const KEYWORDS: &[&str] = &["true", "false", "inf", "NaN"];

pub struct Deserializer<'de> {
    source: &'de str,
    offset: usize,
}

impl<'de> Deserializer<'de> {
    #[inline]
    pub fn new(source: &'de str) -> Result<Self> {
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
    const fn rest(&self) -> &'de str {
        self.source.split_at(self.offset).1
    }
    #[inline]
    const fn rest_bytes(&self) -> &'de [u8] {
        self.source.as_bytes().split_at(self.offset).1
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
    const fn bump(&mut self, n: usize) -> Option<&'de [u8]> {
        if self.source.is_char_boundary(self.offset + n) {
            let delta = self.rest_bytes().split_at(n).0;
            self.offset += n;
            Some(delta)
        } else {
            None
        }
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
    const fn raise_unexpected_end<T>(&self) -> Result<T> {
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
                .map(|(off, _ch)| off)
                .unwrap_or(0),
        )
        .unwrap()
    }

    #[inline]
    fn consume_ident(&mut self, ident: &'static str) -> Result<bool> {
        let start = self.offset;
        if !self.consume_if(|ch| *ch == '`') {
            if let Some(keyword) = KEYWORDS.iter().find(|kw| **kw == ident) {
                return self.raise_at(start, ErrorKind::UnexpectedKeyword { keyword });
            }
        }

        if self.consume_ws_(ident)? {
            Ok(true)
        } else {
            self.offset = start;
            Ok(false)
        }
    }

    #[inline]
    fn consume_next_ident(&mut self) -> Result<&'de str> {
        let raw_mode = self.consume_if(|ch| *ch == '`');

        let start = self.offset;
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

        let end = self.offset;
        self.consume_whitespace_comment()?;

        let ident = &self.source[start..end];
        if !raw_mode {
            if let Some(keyword) = KEYWORDS.iter().find(|kw| **kw == ident) {
                return self.raise_at(start, ErrorKind::UnexpectedKeyword { keyword });
            }
        }

        Ok(ident)
    }

    #[inline]
    fn consume_whitespace_comment(&mut self) -> Result<()> {
        fn is_whitespace(ch: &char) -> bool {
            ch.is_whitespace()
        }

        loop {
            self.consume_while(is_whitespace);

            if self.consume("//") {
                match memchr::memchr(b'\n', self.rest_bytes()) {
                    Some(off) => {
                        self.bump(off);
                    }
                    None => {
                        self.offset = self.source.len();
                        break;
                    }
                }
            } else if self.consume("/*") {
                let mut depth = 1u8;

                while depth != 0 {
                    let Some(off) = memchr::memchr(b'/', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
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
        match self.peek_byte().and_then(|byte| {
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
            None => self.raise(ErrorKind::InvalidEscape),
            Some(byte) => {
                self.bump(1);
                Ok(byte)
            }
        }
    }

    #[inline]
    fn __escape_byte(&mut self) -> Option<Result<u8>> {
        if self.consume("x") {
            Some({
                if let Some(delta) = self.bump(2) {
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
                if let Some(delta) = self.bump(2) {
                    if matches!(delta[0], b'0'..=b'7') && delta[1].is_ascii_hexdigit() {
                        return Some(Ok(lexical_core::parse::<u8>(delta).unwrap().into()));
                    }
                }
                self.raise(ErrorKind::InvalidAsciiEscape)
            })
        } else {
            self.consume("u").then(|| {
                let off = self.offset;
                let delta = self.consume_while(|ch| *ch != '}');
                let Some((b'{', delta)) = delta.split_first() else {
                    return self.raise_at(off, ErrorKind::Expected("`{`"));
                };
                let chr = lexical_core::parse_with_options::<
                    u32,
                    {
                        NumberFormatBuilder::rebuild(lexical_core::format::RUST_LITERAL)
                            .mantissa_radix(16)
                            .build()
                    },
                >(delta, &PARSE_INTEGER_OPTS)
                .or_else(|e| self.raise_at(off + 1, e.into()))?;

                if self.consume("}") {
                    if let Some(chr) = char::from_u32(chr) {
                        Ok(chr)
                    } else {
                        self.raise_at(off + 1, ErrorKind::InvalidUnicodeEscape)
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

    #[inline]
    fn access_tuple<'a>(&'a mut self, req_trailing_comma: bool) -> SeqAccessor<'a, 'de, false> {
        SeqAccessor {
            der: self,
            req_trailing_comma,
        }
    }

    #[inline]
    fn access_seq<'a>(&'a mut self) -> SeqAccessor<'a, 'de, true> {
        SeqAccessor {
            der: self,
            req_trailing_comma: false,
        }
    }

    #[inline]
    fn access_map<'a>(&'a mut self) -> MapAccessor<'a, 'de, false> {
        MapAccessor { der: self }
    }

    #[inline]
    fn access_struct<'a>(&'a mut self) -> MapAccessor<'a, 'de, true> {
        MapAccessor { der: self }
    }

    #[inline]
    fn access_enum<'a>(&'a mut self, variant: &'de str) -> EnumAccessor<'a, 'de> {
        EnumAccessor { der: self, variant }
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
        let (x, o) = if matches!(rest, [b'0', b'x', ..] | [b'-', b'0', b'x', ..]) {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_HEX>(rest, &PARSE_INTEGER_OPTS)
        } else if matches!(rest, [b'0', b'o', ..] | [b'-', b'0', b'o', ..]) {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_OCT>(rest, &PARSE_INTEGER_OPTS)
        } else if matches!(rest, [b'0', b'b', ..] | [b'-', b'0', b'b', ..]) {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_BIN>(rest, &PARSE_INTEGER_OPTS)
        } else {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT>(rest, &PARSE_INTEGER_OPTS)
        }
        .or_else(|e| $self.raise(e.into()))?;

        $self.bump(o);
        $self.consume_whitespace_comment()?;
        $visitor.$method(x)
    }};
}

macro_rules! deserialize_float {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        let (x, o) =
            lexical_core::parse_partial_with_options::<$ty, FLOAT_FORMAT>($self.rest_bytes(), &PARSE_FLOAT_OPTS)
                .or_else(|e| $self.raise(e.into()))?;
        $self.bump(o);
        $self.consume_whitespace_comment()?;
        $visitor.$method(x)
    }};
}

macro_rules! maybe_deserialize_baseXX {
    ( $self:ident, $indicator:literal, $decoder:ident, $visitor:ident ) => {{
        if $self.consume($indicator) {
            let Some(off) = ::memchr::memchr(b'"', $self.rest_bytes()) else {
                return $self.raise_unexpected_end();
            };

            let buf = $decoder.decode(&$self.rest_bytes()[..off]).map_err(|e| {
                let mut e: Error = e.into();
                e.pos += $self.offset;
                e
            })?;

            $self.bump(off + 1);
            $self.consume_whitespace_comment()?;

            return $visitor.visit_byte_buf(buf);
        }
    }};
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _vis: V) -> Result<V::Value> {
        self.raise(ErrorKind::WontImplement)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, _vis: V) -> Result<V::Value> {
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
            let start = self.offset;
            'outer: {
                let byte = match self.escape_byte() {
                    Some(byte) => byte?,
                    None => match self.rest().chars().next() {
                        None => break 'outer,
                        Some(ch) => match ch.try_into() {
                            Ok(byte) => byte,
                            Err(_) => break 'outer,
                        },
                    },
                };

                if !self.consume_ws_("'")? {
                    return self.raise(ErrorKind::Expected("`'`"));
                }

                return vis.visit_u8(byte);
            }

            self.raise_at(start, ErrorKind::ExpectedByteInteger)
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
        let start = self.offset;
        'outer: {
            if !self.consume("'") {
                break 'outer;
            }

            let ch = match self.escape_char() {
                Some(ch) => ch?,
                None => match self.rest().chars().next() {
                    Some(ch) => ch,
                    None => break 'outer,
                },
            };

            if !self.consume_ws_("'")? {
                return self.raise(ErrorKind::Expected("`'`"));
            }

            return vis.visit_char(ch);
        }

        self.raise_at(start, ErrorKind::ExpectedCharacter)
    }

    fn deserialize_string<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_str(vis)
    }
    fn deserialize_str<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if self.consume("|") {
            /* paragraph */
            const fn trim_line(s: &str) -> &str {
                if let Some((b' ', s)) = s.as_bytes().split_first() {
                    unsafe { str::from_utf8_unchecked(s) }
                } else {
                    s
                }
                .trim_ascii_end()
            }

            let mut buf = String::new();
            let mut start = self.offset;
            let end = loop {
                let end = match memchr::memchr(b'\n', self.rest_bytes()) {
                    Some(off) => {
                        self.bump(1);
                        off
                    }
                    None => {
                        self.offset = self.source.len();
                        break self.offset;
                    }
                };

                self.consume_while(|&ch| ch != '\n' && ch.is_whitespace());
                match self.peek_byte() {
                    byte @ (Some(b'|') | Some(b'<') | Some(b'>')) => {
                        self.bump(1);
                        buf.push_str(trim_line(&self.source[start..end]));
                        match byte {
                            Some(b'|') => buf.push_str("\n"),
                            Some(b'>') => buf.push_str(" "),
                            Some(b'<') => buf.push_str(""),
                            _ => unreachable!(),
                        }
                    }

                    Some(b'\n') => {
                        self.consume_whitespace_comment()?;
                        if matches!(self.peek_byte(), Some(b'|') | Some(b'<') | Some(b'>')) {
                            return self.raise_at(end, ErrorKind::BrokenParagraph);
                        }
                        break end;
                    }

                    _ => break end,
                }

                start = self.offset;
            };

            if buf.is_empty() {
                return vis.visit_borrowed_str(trim_line(&self.source[start..end]));
            } else {
                return vis.visit_string(buf);
            }
        }

        let start = self.offset;
        'outer: {
            /* string */
            let enclosure = self.consume_while(|ch| *ch == '`').len();
            if enclosure >= u8::MAX as _ {
                return self.raise(ErrorKind::ThickRawEnclosure);
            }

            if !self.consume("\"") {
                break 'outer;
            }

            let inner_start = self.offset;
            if enclosure > 0 {
                /* raw string */
                loop {
                    let Some(off) = memchr::memchr(b'"', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    self.bump(off + 1);

                    let inner_end = self.offset - 1;
                    return match self.consume_while(|ch| *ch == '`').len().cmp(&enclosure) {
                        Ordering::Less => continue,
                        Ordering::Equal => {
                            self.consume_whitespace_comment()?;
                            vis.visit_borrowed_str(&self.source[inner_start..inner_end])
                        }
                        Ordering::Greater => self.raise_at(inner_end + 2, ErrorKind::ThickRawEnclosure),
                    };
                }
            } else {
                /* normal string */
                let mut buf = String::new();
                let mut cursor = self.offset;

                loop {
                    let Some(off) = memchr::memchr3(b'\\', b'\"', b'\n', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    self.bump(off);
                    match self.peek_byte().unwrap() {
                        b'\\' => {
                            let str = &self.source[cursor..self.offset];
                            let ch = self.escape_char().unwrap()?;
                            cursor = self.offset;
                            buf.push_str(str);
                            buf.push(ch);
                        }
                        b'\"' => {
                            self.bump(1);
                            break;
                        }
                        b'\n' => return self.raise(ErrorKind::MultilineNormalString),

                        _ => unreachable!(),
                    }
                }

                let inner_end = self.offset - 1;
                self.consume_whitespace_comment()?;

                if buf.is_empty() {
                    return vis.visit_borrowed_str(&self.source[inner_start..inner_end]);
                } else {
                    return vis.visit_string(buf);
                }
            }
        }

        self.raise_at(start, ErrorKind::ExpectedString)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_bytes(vis)
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let start = self.offset;
        'outer: {
            let pure_ascii = |s: &'de str| -> Result<&'de [u8]> {
                s.is_ascii()
                    .then_some(s.as_bytes())
                    .ok_or_else(|| Error::new_at(start, ErrorKind::NonAsciiByteString))
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
                        return self.raise_unexpected_end();
                    };

                    self.bump(off + 1);

                    let inner_end = self.offset - 1;
                    return match self.consume_while(|ch| *ch == '`').len().cmp(&enclosure) {
                        Ordering::Less => continue,
                        Ordering::Equal => {
                            self.consume_whitespace_comment()?;
                            vis.visit_borrowed_bytes(pure_ascii(&self.source[inner_start..inner_end])?)
                        }
                        Ordering::Greater => self.raise_at(inner_end + 2, ErrorKind::ThickRawEnclosure),
                    };
                }
            } else {
                /* normal bytes */
                let mut buf = Vec::new();
                let mut cursor = self.offset;

                loop {
                    let Some(off) = memchr::memchr3(b'\\', b'\"', b'\n', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    self.bump(off);
                    match self.peek_byte().unwrap() {
                        b'\\' => {
                            let bytes = pure_ascii(&self.source[cursor..self.offset])?;
                            let byte = self.escape_byte().unwrap()?;
                            cursor = self.offset;
                            buf.extend_from_slice(bytes);
                            buf.push(byte);
                        }
                        b'\"' => {
                            self.bump(1);
                            break;
                        }
                        b'\n' => return self.raise(ErrorKind::MultilineNormalString),

                        _ => unreachable!(),
                    }
                }

                let inner_end = self.offset - 1;
                self.consume_whitespace_comment()?;

                if buf.is_empty() {
                    return vis.visit_borrowed_bytes(pure_ascii(&self.source[inner_start..inner_end])?);
                } else {
                    return vis.visit_byte_buf(buf);
                }
            }
        }

        self.raise_at(start, ErrorKind::ExpectedByteString)
    }

    fn deserialize_option<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if self.consume_ws_("?")? {
            if self.adjacent_to_delim() {
                vis.visit_none()
            } else {
                vis.visit_some(self)
            }
        } else {
            self.raise(ErrorKind::ExpectedOption)
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
        if self.consume_ws_("(")? {
            self.consume_ident(name)?;
            if self.consume_ws_(")")? {
                return vis.visit_unit();
            }
        }

        self.raise_at(start, ErrorKind::ExpectedUnitStruct { name })
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if (self.consume_ws_("_")? || self.consume_ws_("(")? && self.consume_ident(name)? && self.consume_ws_(")")?)
            && self.consume_ws_("(")?
        {
            let val = vis.visit_newtype_struct(&mut *self)?;
            self.consume_ws_(",")?;
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::Expected("`)` and optional preceding `,`"));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedNewtypeStruct { name })
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, _len: usize, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if (self.consume_ws_("_")? || self.consume_ws_("(")? && self.consume_ident(name)? && self.consume_ws_(")")?)
            && self.consume_ws_("(")?
        {
            let val = vis.visit_seq(self.access_tuple(false))?;
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::Expected("`)`"));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedTupleStruct { name })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        let start = self.offset;
        if (self.consume_ws_("_")? || self.consume_ws_("(")? && self.consume_ident(name)? && self.consume_ws_(")")?)
            && self.consume_ws_("{")?
        {
            let val = vis.visit_map(self.access_struct())?;
            if !self.consume_ws_("}")? {
                return self.raise(ErrorKind::Expected("`}`"));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedStruct { name })
    }

    //------------------------------------------------------------------------------

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if self.consume_ws_("(")? {
            let val = vis.visit_seq(self.access_tuple(len == 1))?;
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::Expected("`)`"));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedTuple)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if self.consume_ws_("[")? {
            let val = vis.visit_seq(self.access_seq())?;
            if !self.consume_ws_("]")? {
                return self.raise(ErrorKind::Expected("`]`"));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedSequence)
    }

    fn deserialize_map<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let start = self.offset;
        if self.consume_ws_("{")? {
            let val = vis.visit_map(self.access_map())?;
            if !self.consume_ws_("}")? {
                return self.raise(ErrorKind::Expected("`}`"));
            }

            return Ok(val);
        }

        self.raise_at(start, ErrorKind::ExpectedMap)
    }

    //------------------------------------------------------------------------------

    fn deserialize_identifier<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        vis.visit_borrowed_str(self.consume_next_ident()?)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        let mut start = self.offset;
        let mut variant = self.consume_next_ident()?;

        if self.consume_ws_("::")? {
            if variant != name {
                return self.raise_at(start, ErrorKind::ExpectedEnum { name });
            }

            start = self.offset;
            variant = self.consume_next_ident()?;
        }

        if !variants.contains(&variant) {
            return self.raise_at(start, ErrorKind::ExpectedVariant { variants });
        }

        vis.visit_enum(self.access_enum(variant))
    }
}

//------------------------------------------------------------------------------

struct SeqAccessor<'a, 'de, const VECTOR_MODE: bool> {
    der: &'a mut Deserializer<'de>,
    req_trailing_comma: bool,
}

impl<'a, 'de, const VECTOR_MODE: bool> Deref for SeqAccessor<'a, 'de, VECTOR_MODE> {
    type Target = Deserializer<'de>;
    fn deref(&self) -> &Self::Target {
        self.der
    }
}

impl<'a, 'de, const VECTOR_MODE: bool> DerefMut for SeqAccessor<'a, 'de, VECTOR_MODE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.der
    }
}

impl<'a, 'de, const VECTOR_MODE: bool> SeqAccess<'de> for SeqAccessor<'a, 'de, VECTOR_MODE> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.adjacent_to_delim() {
            return Ok(None);
        }

        let val = seed.deserialize(&mut **self)?;

        if !self.consume_ws_(",")? {
            if VECTOR_MODE {
                if !matches!(self.peek_byte(), Some(b']')) {
                    return self.raise(ErrorKind::Expected("`,` or `]`"));
                }
            } else if !matches!(self.peek_byte(), Some(b')')) {
                return self.raise(ErrorKind::Expected("`,` or `)`"));
            } else if self.req_trailing_comma {
                return self.raise(ErrorKind::Expected("`,` for a tuple of length 1"));
            }
        }

        Ok(Some(val))
    }
}

//------------------------------------------------------------------------------

struct MapAccessor<'a, 'de, const STRUCT_MODE: bool> {
    der: &'a mut Deserializer<'de>,
}

impl<'a, 'de, const STRUCT_MODE: bool> Deref for MapAccessor<'a, 'de, STRUCT_MODE> {
    type Target = Deserializer<'de>;
    fn deref(&self) -> &Self::Target {
        self.der
    }
}

impl<'a, 'de, const STRUCT_MODE: bool> DerefMut for MapAccessor<'a, 'de, STRUCT_MODE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.der
    }
}

impl<'a, 'de, const STRUCT_MODE: bool> MapAccess<'de> for MapAccessor<'a, 'de, STRUCT_MODE> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.adjacent_to_delim() {
            return Ok(None);
        }

        let val = seed.deserialize(&mut **self)?;

        if STRUCT_MODE {
            if !self.consume_ws_(":")? {
                return self.raise(ErrorKind::Expected("`:`"));
            }
        } else if !self.consume_ws_("=>")? {
            return self.raise(ErrorKind::Expected("`=>`"));
        }

        Ok(Some(val))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let val = seed.deserialize(&mut **self)?;

        if !self.consume_ws_(",")? && !matches!(self.peek_byte(), Some(b'}')) {
            return self.raise(ErrorKind::Expected("`,` or `}`"));
        }

        Ok(val)
    }
}

//------------------------------------------------------------------------------

struct EnumAccessor<'a, 'de> {
    der: &'a mut Deserializer<'de>,
    variant: &'de str,
}

impl<'a, 'de> EnumAccess<'de> for EnumAccessor<'a, 'de> {
    type Error = Error;

    type Variant = &'a mut Deserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> std::result::Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        Ok((
            seed.deserialize(BorrowedStrDeserializer::<Error>::new(self.variant))?,
            self.der,
        ))
    }
}

impl<'de> VariantAccess<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        if !self.adjacent_to_delim() {
            return self.raise(ErrorKind::ExpectedUnitVariant);
        }

        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        if !self.consume_ws_("(")? {
            return self.raise(ErrorKind::ExpectedNewtypeVariant);
        }

        let val = seed.deserialize(&mut *self)?;
        self.consume_ws_(",")?;
        if !self.consume_ws_(")")? {
            return self.raise(ErrorKind::Expected("`)` and optional preceding `,`"));
        }

        Ok(val)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, vis: V) -> Result<V::Value> {
        if !self.consume_ws_("(")? {
            return self.raise(ErrorKind::ExpectedNewtypeVariant);
        }

        let val = vis.visit_seq(self.access_tuple(false))?;
        if !self.consume_ws_(")")? {
            return self.raise(ErrorKind::Expected("`)`"));
        }

        Ok(val)
    }

    fn struct_variant<V: Visitor<'de>>(self, _fields: &'static [&'static str], vis: V) -> Result<V::Value> {
        if !self.consume_ws_("{")? {
            return self.raise(ErrorKind::ExpectedStructVariant);
        }

        let val = vis.visit_map(self.access_struct())?;
        if !self.consume_ws_("}")? {
            return self.raise(ErrorKind::Expected("}"));
        }

        Ok(val)
    }
}
