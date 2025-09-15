use super::*;
use core::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use serde::{
    de::{value::BorrowedStrDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserializer,
};

impl<'de> Parser<'de> {
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

//------------------------------------------------------------------------------

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
        let val = $visitor.$method::<Error>(x)?;
        $self.consume_whitespace_comment()?;

        return Ok(val);
    }};
}

macro_rules! deserialize_float {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        let (x, o) =
            lexical_core::parse_partial_with_options::<$ty, FLOAT_FORMAT>($self.rest_bytes(), &PARSE_FLOAT_OPTS)
                .or_else(|e| $self.raise(e.into()))?;

        $self.bump(o);
        let val = $visitor.$method::<Error>(x)?;
        $self.consume_whitespace_comment()?;

        return Ok(val);
    }};
}

macro_rules! maybe_deserialize_baseXX {
    ( $self:ident, $indicator:literal, $decoder:ident, $visitor:ident ) => {{
        if $self.consume($indicator) {
            let Some(off) = memchr::memchr(b'"', $self.rest_bytes()) else {
                return $self.raise_unexpected_end();
            };

            let buf = $decoder.decode(&$self.rest_bytes()[..off]).map_err(|e| {
                let mut e: Error = e.into();
                e.pos += $self.pos;
                e
            })?;

            $self.bump(off + 1);
            let val = $visitor.visit_byte_buf::<Error>(buf)?;
            $self.consume_whitespace_comment()?;

            return Ok(val);
        }
    }};
}

//------------------------------------------------------------------------------

impl<'de> Deserializer<'de> for &mut Parser<'de> {
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
            let start = self.pos;
            'byte: {
                let byte = match self.escape_byte() {
                    Some(byte) => byte?,
                    None => {
                        let Some(ch) = self.rest().chars().next() else {
                            break 'byte;
                        };
                        let Ok(byte) = ch.try_into() else {
                            break 'byte;
                        };
                        self.bump(1);
                        byte
                    }
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
        vis.visit_char(self.parse_char()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_str(vis)
    }
    fn deserialize_str<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let start = self.pos;
        let enclosure = self.consume_while_fast(is_backtick).len();

        if self.consume("\"") {
            /* string */
            let mut buf = String::new();
            let mut cursor = self.pos; // initialized as `inner_start`
            let mut inner_end;

            match enclosure > 0 {
                // raw string
                true => loop {
                    let Some(off) = memchr::memchr2(b'\"', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    self.bump(off);

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            buf.push_str(&self.src[cursor..self.pos]);
                            buf.push('\n');
                            self.consume_newline()?;
                        }
                        b'\"' => {
                            inner_end = self.pos;
                            self.bump(1);
                            match self.consume_while_fast(is_backtick).len().cmp(&enclosure) {
                                Ordering::Less => continue,
                                Ordering::Equal => break,
                                Ordering::Greater => {
                                    return self.raise_at(inner_end + 1, ErrorKind::UnbalancedRawEnclosure)
                                }
                            }
                        }
                        _ => unreachable!(),
                    }

                    cursor = self.pos;
                },

                // normal string
                false => loop {
                    let Some(off) = memchr::memchr3(b'\"', b'\\', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    self.bump(off);

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            buf.push_str(&self.src[cursor..self.pos]);
                            buf.push('\n');
                            self.consume_newline()?;
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
            }

            let last = &self.src[cursor..inner_end];
            let val = if buf.is_empty() {
                vis.visit_borrowed_str::<Error>(last)?
            } else {
                buf.push_str(last);
                vis.visit_string::<Error>(buf)?
            };

            self.consume_whitespace_comment()?;

            Ok(val)
        } else if enclosure > 0 && self.consume("|") {
            /* paragraph */
            fn trim(s: &str) -> &str {
                if let Some((b' ', s)) = s.as_bytes().split_first() {
                    unsafe { core::str::from_utf8_unchecked(s) }
                } else {
                    s
                }
                .trim_end()
            }

            let mut buf = String::new();
            let mut first;
            match memchr::memchr2(b'\r', b'\n', self.rest_bytes()) {
                None => first = Some(trim(self.bump_to_end())),
                Some(off) => {
                    first = Some(trim(self.bump(off)));
                    self.consume_newline()?;
                    self.consume_whitespace_comment()?;
                }
            }

            loop {
                if self.adjacent_to_delim() {
                    break;
                }
                if self.consume_while_fast(is_backtick).len() != enclosure {
                    return self.raise(ErrorKind::UnbalancedRawEnclosure);
                }
                let Some(sym) = self.peek_byte() else {
                    return self.raise(ErrorKind::InvalidIndicator);
                };
                if !matches!(sym, b'|' | b'<' | b'>') {
                    return self.raise(ErrorKind::InvalidIndicator);
                }

                self.bump(1);

                let conti;
                match memchr::memchr2(b'\r', b'\n', self.rest_bytes()) {
                    None => conti = trim(self.bump_to_end()),
                    Some(off) => {
                        conti = trim(self.bump(off));
                        self.consume_newline()?;
                        self.consume_whitespace_comment()?;
                    }
                }

                if let Some(first) = first.take() {
                    buf.push_str(first);
                }
                if sym == b'|' {
                    buf.push('\n');
                }
                if sym == b'>' && !conti.is_empty() {
                    buf.push(' ');
                }

                buf.push_str(conti);
            }

            match first {
                Some(s) => vis.visit_borrowed_str(s),
                None => vis.visit_string(buf),
            }
        } else {
            self.raise_at(start, ErrorKind::ExpectedString)
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_bytes(vis)
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let start = self.pos;
        'byte_string: {
            fn filter_non_ascii(s: &str, de: &mut Parser) -> Result<()> {
                match s.as_bytes().iter().enumerate().find(|(_off, byte)| **byte >= 0x80) {
                    Some((off, _byte)) => de.raise_at(de.pos - s.len() + off, ErrorKind::NonAsciiByteString),
                    None => Ok(()),
                }
            }

            if !self.consume("b") {
                break 'byte_string;
            }

            maybe_deserialize_baseXX!(self, "64\"", BASE64URL_NOPAD, vis);
            maybe_deserialize_baseXX!(self, "32\"", BASE32_NOPAD, vis);
            maybe_deserialize_baseXX!(self, "16\"", HEXUPPER_PERMISSIVE, vis);

            let enclosure = self.consume_while_fast(is_backtick).len();

            if !self.consume("\"") {
                break 'byte_string;
            }

            let mut buf = Vec::new();
            let mut cursor = self.pos; // initialized as `inner_start`
            let mut inner_end;

            match enclosure > 0 {
                // raw bytes
                true => loop {
                    let Some(off) = memchr::memchr2(b'\"', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    filter_non_ascii(self.bump(off), self)?;

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(b'\n');
                            self.consume_newline()?;
                        }
                        b'\"' => {
                            inner_end = self.pos;
                            self.bump(1);
                            match self.consume_while_fast(is_backtick).len().cmp(&enclosure) {
                                Ordering::Less => continue,
                                Ordering::Equal => break,
                                Ordering::Greater => {
                                    return self.raise_at(inner_end + 1, ErrorKind::UnbalancedRawEnclosure)
                                }
                            }
                        }
                        _ => unreachable!(),
                    }

                    cursor = self.pos;
                },

                // normal bytes
                false => loop {
                    let Some(off) = memchr::memchr3(b'\"', b'\\', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    filter_non_ascii(self.bump(off), self)?;

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(b'\n');
                            self.consume_newline()?;
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
            }

            let last = &self.source_bytes()[cursor..inner_end];
            let val = if buf.is_empty() {
                vis.visit_borrowed_bytes::<Error>(last)?
            } else {
                buf.extend_from_slice(last);
                vis.visit_byte_buf::<Error>(buf)?
            };

            self.consume_whitespace_comment()?;

            return Ok(val);
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
        let start = self.pos;
        if self.consume_ws_("(")? && self.consume_ws_(")")? {
            return vis.visit_unit();
        }

        self.raise_at(start, ErrorKind::ExpectedUnit)
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        let start = self.pos;
        if self.consume_ws_("(")? {
            self.consume_ident_exact(name)?;
            if self.consume_ws_(")")? {
                return vis.visit_unit();
            }
        }

        self.raise_at(start, ErrorKind::ExpectedUnitStruct { name })
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        let start = self.pos;
        if (self.consume_ws_("_")?
            || self.consume_ws_("(")? && self.consume_ident_exact(name)? && self.consume_ws_(")")?)
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
        let start = self.pos;
        if (self.consume_ws_("_")?
            || self.consume_ws_("(")? && self.consume_ident_exact(name)? && self.consume_ws_(")")?)
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
        let start = self.pos;
        if (self.consume_ws_("_")?
            || self.consume_ws_("(")? && self.consume_ident_exact(name)? && self.consume_ws_(")")?)
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
        let start = self.pos;
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
        let start = self.pos;
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
        let start = self.pos;
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
        vis.visit_borrowed_str(self.consume_ident()?)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        let mut start = self.pos;
        let mut variant = self.consume_ident()?;

        if self.consume_ws_("::")? {
            if variant != name {
                return self.raise_at(start, ErrorKind::ExpectedEnum { name });
            }

            start = self.pos;
            variant = self.consume_ident()?;
        }

        if !variants.contains(&variant) {
            return self.raise_at(start, ErrorKind::ExpectedVariant { variants });
        }

        vis.visit_enum(self.access_enum(variant))
    }
}

//------------------------------------------------------------------------------

struct SeqAccessor<'a, 'de, const VECTOR_MODE: bool> {
    der: &'a mut Parser<'de>,
    req_trailing_comma: bool,
}

impl<'a, 'de, const VECTOR_MODE: bool> Deref for SeqAccessor<'a, 'de, VECTOR_MODE> {
    type Target = Parser<'de>;
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
    der: &'a mut Parser<'de>,
}

impl<'a, 'de, const STRUCT_MODE: bool> Deref for MapAccessor<'a, 'de, STRUCT_MODE> {
    type Target = Parser<'de>;
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
    der: &'a mut Parser<'de>,
    variant: &'de str,
}

impl<'a, 'de> EnumAccess<'de> for EnumAccessor<'a, 'de> {
    type Error = Error;

    type Variant = &'a mut Parser<'de>;

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

impl<'de> VariantAccess<'de> for &mut Parser<'de> {
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
