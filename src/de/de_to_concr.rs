use super::*;
use serde::{
    de::{value::BorrowedStrDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

pub fn parse<'de, T>(s: &'de str) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut der = Parser::new(s);
    let value = T::deserialize(&mut der)?;
    der.finish().and(Ok(value))
}

pub fn parse_many<'de, T>(s: &'de str) -> IterParser<'de, T>
where
    T: Deserialize<'de>,
{
    Parser::new(s).into_iter()
}

//------------------------------------------------------------------------------

/// NOTE:
/// As an iterator, once the internal parser becomes corrupted,
/// it will always return `Some(Err(_))` with [`ErrorKind::Corrupted`] .
pub struct IterParser<'de, T> {
    der: Parser<'de>,
    phantom: PhantomData<T>,
}

impl<'de, T> Iterator for IterParser<'de, T>
where
    T: Deserialize<'de>,
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
            if let Err(e) = self.der.consume_whitespace_comment_first() {
                break 'fail e;
            }
            let v = match T::deserialize(&mut self.der) {
                Err(e) => break 'fail e,
                Ok(v) => v,
            };
            if let Err(e) = self.der.finish_one() {
                break 'fail e;
            }

            return Some(Ok(v));
        };

        Some(Err(e))
    }
}

impl<'de, T> IterParser<'de, T>
where
    T: Deserialize<'de>,
{
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
}

impl<'de> Parser<'de> {
    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter<T>(self) -> IterParser<'de, T>
    where
        T: Deserialize<'de>,
    {
        IterParser {
            der: self,
            phantom: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------

macro_rules! deserialize_integer {
    ( $self:ident, $ty:ident ) => {{
        $self.corrupt_guard()?;

        let rest = $self.rest_bytes();
        if rest.starts_with(b"0x") {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_HEX>(rest, &PARSE_INTEGER_OPTS)
        } else if rest.starts_with(b"0o") {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_OCT>(rest, &PARSE_INTEGER_OPTS)
        } else if rest.starts_with(b"0b") {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT_BIN>(rest, &PARSE_INTEGER_OPTS)
        } else {
            lexical_core::parse_partial_with_options::<$ty, INTEGER_FORMAT>(rest, &PARSE_INTEGER_OPTS)
        }
        .or_else(|e| $self.raise(e.into()))
    }};
}

macro_rules! deserialize_unsigned_integer {
    ( $self:ident, $ty:ident, $visitor:ident, $method:ident ) => {{
        let (num, off) = deserialize_integer!($self, $ty)?;
        let val = $self.watch($visitor.$method(num))?;

        $self.bump(off);
        $self.consume_whitespace_comment()?;

        Ok(val)
    }};
}

macro_rules! deserialize_signed_integer {
    ( $self:ident, $ty:ident, $out:ident, $visitor:ident, $method:ident ) => {{
        let start = $self.pos;
        let neg = $self.consume_ws_("-")?;
        let (num, off) = deserialize_integer!($self, $ty)?;

        let num = if neg {
            if num <= $out::MIN.unsigned_abs() {
                Ok((!num).wrapping_add(1) as $out)
            } else {
                $self.raise_at(start, ErrorKind::IntegerUnderflow)
            }
        } else if num > $out::MAX as $ty {
            $self.raise_at(start, ErrorKind::IntegerOverflow)
        } else {
            Ok(num as $out)
        }?;

        let val = $self.watch($visitor.$method(num))?;

        $self.bump(off);
        $self.consume_whitespace_comment()?;

        Ok(val)
    }};
}

macro_rules! deserialize_float {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        $self.corrupt_guard()?;

        let start = $self.pos;
        let neg = $self.consume_ws_("-")?;
        let (num, off) =
            lexical_core::parse_partial_with_options::<$ty, FLOAT_FORMAT>($self.rest_bytes(), &PARSE_FLOAT_OPTS)
                .or_else(|e| $self.raise_at(start, e.into()))?;

        let num = if neg { -num } else { num };
        let val = $self.watch($visitor.$method(num))?;

        $self.bump(off);
        $self.consume_whitespace_comment()?;

        Ok(val)
    }};
}

impl<'de> Deserializer<'de> for &mut Parser<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _vis: V) -> Result<V::Value> {
        self.raise(ErrorKind::WontImplement)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, _vis: V) -> Result<V::Value> {
        self.raise(ErrorKind::WontImplement)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        if self.consume_ws_("true")? {
            self.watch(vis.visit_bool(true))
        } else if self.consume_ws_("false")? {
            self.watch(vis.visit_bool(false))
        } else {
            self.raise(ErrorKind::ExpectedBoolean)
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        let res = vis.visit_char(self.parse_char()?);
        self.watch(res)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if let Some(b'b') = self.peek_byte() {
            self.corrupt_guard()?;
            let res = vis.visit_u8(self.parse_byte()?);
            self.watch(res)
        } else {
            deserialize_unsigned_integer!(self, u8, vis, visit_u8)
        }
    }
    fn deserialize_u16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_unsigned_integer!(self, u16, vis, visit_u16)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_unsigned_integer!(self, u32, vis, visit_u32)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_unsigned_integer!(self, u64, vis, visit_u64)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_unsigned_integer!(self, u128, vis, visit_u128)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_signed_integer!(self, u8, i8, vis, visit_i8)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_signed_integer!(self, u16, i16, vis, visit_i16)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_signed_integer!(self, u32, i32, vis, visit_i32)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_signed_integer!(self, u64, i64, vis, visit_i64)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_signed_integer!(self, u128, i128, vis, visit_i128)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_float!(self, f32, vis, visit_f32)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_float!(self, f64, vis, visit_f64)
    }

    fn deserialize_string<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_str(vis)
    }
    fn deserialize_str<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        match self.parse_string_or_paragraph()? {
            Either::Left(s) => self.watch(vis.visit_borrowed_str(s)),
            Either::Right(buf) => self.watch(vis.visit_string(buf)),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_bytes(vis)
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        match self.parse_byte_string()? {
            Either::Left(bytes) => self.watch(vis.visit_borrowed_bytes(bytes)),
            Either::Right(buf) => self.watch(vis.visit_byte_buf(buf)),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        if self.consume_ws_("?")? {
            if self.adjacent_to_delim() {
                self.watch(vis.visit_none())
            } else {
                let res = vis.visit_some(&mut *self);
                self.watch(res)
            }
        } else {
            self.raise(ErrorKind::ExpectedMaybe)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        let start = self.pos;
        if self.consume_ws_("(")? && self.consume_ws_(")")? {
            self.watch(vis.visit_unit())
        } else {
            self.raise_at(start, ErrorKind::ExpectedUnit)
        }
    }

    //------------------------------------------------------------------------------

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        if self.consume_ws_("(")? {
            let res = vis.visit_seq(self.access_tuple());
            let val = self.watch(res)?;

            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::Expected("`)`"));
            }

            Ok(val)
        } else {
            self.raise(ErrorKind::ExpectedTuple)
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        if self.consume_ws_("[")? {
            let res = vis.visit_seq(self.access_seq());
            let val = self.watch(res)?;

            if !self.consume_ws_("]")? {
                return self.raise(ErrorKind::Expected("`]`"));
            }

            Ok(val)
        } else {
            self.raise(ErrorKind::ExpectedSequence)
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        if self.consume_ws_("{")? {
            let res = vis.visit_map(self.access_map());
            let val = self.watch(res)?;

            if !self.consume_ws_("}")? {
                return self.raise(ErrorKind::Expected("`}`"));
            }

            Ok(val)
        } else {
            self.raise(ErrorKind::ExpectedMap)
        }
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        if self.consume_nominal_path_of_struct(name)? {
            /* Name */
            if self.consume_ws_("(")? {
                /* Name () */
                if !self.consume_ws_(")")? {
                    return self.raise(ErrorKind::Expected("`)`"));
                }
            }
        } else if self.consume_ws_("(")? {
            /* () */
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::Expected("`)`"));
            }
        } else {
            return self.raise(ErrorKind::ExpectedUnitStruct { name });
        }

        self.watch(vis.visit_unit())
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        let start = self.pos;
        if self.consume_nominal_path_of_struct(name)? && self.consume_ws_("(")? {
            let res = vis.visit_newtype_struct(&mut *self);
            let val = self.watch(res)?;

            self.consume_ws_(",")?;
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::Expected("`)`"));
            }

            Ok(val)
        } else {
            self.raise_at(start, ErrorKind::ExpectedNewtypeStruct { name })
        }
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, _len: usize, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        let start = self.pos;
        if self.consume_nominal_path_of_struct(name)? && self.consume_ws_("(")? {
            let res = vis.visit_seq(self.access_tuple());
            let val = self.watch(res)?;

            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::Expected("`)`"));
            }

            Ok(val)
        } else {
            self.raise_at(start, ErrorKind::ExpectedTupleStruct { name })
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        _fields: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        self.corrupt_guard()?;

        let start = self.pos;
        if self.consume_nominal_path_of_struct(name)? && self.consume_ws_("{")? {
            let res = vis.visit_map(self.access_struct());
            let val = self.watch(res)?;

            if !self.consume_ws_("}")? {
                return self.raise(ErrorKind::Expected("`}`"));
            }

            Ok(val)
        } else {
            self.raise_at(start, ErrorKind::ExpectedStruct { name })
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.corrupt_guard()?;

        let res = vis.visit_borrowed_str(self.consume_ident()?);
        self.watch(res)
    }

    //------------------------------------------------------------------------------

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        self.corrupt_guard()?;

        let start = self.pos;
        let Some(variant_name) = self.consume_nominal_path_of_enum(name)? else {
            return self.raise_at(start, ErrorKind::ExpectedEnum { name });
        };

        if !variants.contains(&variant_name) {
            return self.raise_at(start, ErrorKind::ExpectedVariant { variants });
        }

        vis.visit_enum(self.access_enum(variant_name))
    }
}

//------------------------------------------------------------------------------

impl<'de> Parser<'de> {
    fn access_tuple<'a>(&'a mut self) -> SeqAccessor<'a, 'de> {
        SeqAccessor {
            der: (!matches!(self.peek_byte(), Some(b')'))).then_some(self),
        }
    }

    fn access_seq<'a>(&'a mut self) -> SeqAccessor<'a, 'de> {
        SeqAccessor {
            der: (!matches!(self.peek_byte(), Some(b']'))).then_some(self),
        }
    }

    fn access_map<'a>(&'a mut self) -> MapAccessor<'a, 'de, false> {
        MapAccessor {
            der: (!matches!(self.peek_byte(), Some(b'}'))).then_some(self),
        }
    }

    fn access_struct<'a>(&'a mut self) -> MapAccessor<'a, 'de, true> {
        MapAccessor {
            der: (!matches!(self.peek_byte(), Some(b'}'))).then_some(self),
        }
    }

    fn access_enum<'a>(&'a mut self, variant_name: &'de str) -> EnumAccessor<'a, 'de> {
        EnumAccessor {
            der: self,
            variant_name,
        }
    }
}

//------------------------------------------------------------------------------

struct SeqAccessor<'a, 'de> {
    der: Option<&'a mut Parser<'de>>,
}

impl<'a, 'de> SeqAccess<'de> for SeqAccessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        let Some(ref mut der) = self.der else {
            return Ok(None);
        };

        // NOTE:
        // The result of `seed.deserialize(_)` does not need to call `Parser::watch(_)`,
        // as the parser's implementation of `serde::Deserializer` already handles the
        // `corrupted` flag correctly. Same below.
        let val = seed.deserialize(&mut **der)?;
        if !der.consume_ws_(",")? {
            self.der = None;
        }

        Ok(Some(val))
    }
}

//------------------------------------------------------------------------------

struct MapAccessor<'a, 'de, const STRUCT_MODE: bool> {
    der: Option<&'a mut Parser<'de>>,
}

impl<'a, 'de, const STRUCT_MODE: bool> MapAccess<'de> for MapAccessor<'a, 'de, STRUCT_MODE> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        let Some(ref mut der) = self.der else {
            return Ok(None);
        };

        // NOTE:
        // This line of code calls `deserialize_identifier` under STRUCT_MODE,
        // because we are deserializeing the name of a struct field.
        let key = seed.deserialize(&mut **der)?;
        match STRUCT_MODE {
            true => {
                if !der.consume_ws_(":")? {
                    return der.raise(ErrorKind::Expected("`:`"));
                }
            }
            false => {
                if !der.consume_ws_("=>")? {
                    return der.raise(ErrorKind::Expected("`=>`"));
                }
            }
        }

        Ok(Some(key))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let Some(ref mut der) = self.der else {
            panic!("contract violation")
        };

        let val = seed.deserialize(&mut **der)?;
        if !der.consume_ws_(",")? {
            self.der = None;
        }

        Ok(val)
    }
}

//------------------------------------------------------------------------------

struct EnumAccessor<'a, 'de> {
    der: &'a mut Parser<'de>,
    variant_name: &'de str,
}

impl<'a, 'de> EnumAccess<'de> for EnumAccessor<'a, 'de> {
    type Error = Error;
    type Variant = &'a mut Parser<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        Ok((
            // NOTE:
            // This line of code does not call `deserialize_identifier`, because we have
            // complex nominal paths, and variant names have been extracted separately.
            seed.deserialize(BorrowedStrDeserializer::<Error>::new(self.variant_name))?,
            self.der,
        ))
    }
}

impl<'de> VariantAccess<'de> for &mut Parser<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        if self.adjacent_to_delim() {
            Ok(())
        } else {
            self.raise(ErrorKind::ExpectedUnitVariant)
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        if self.consume_ws_("(")? {
            let val = seed.deserialize(&mut *self)?;

            self.consume_ws_(",")?;
            if self.consume_ws_(")")? {
                Ok(val)
            } else {
                self.raise(ErrorKind::Expected("`)`"))
            }
        } else {
            self.raise(ErrorKind::ExpectedNewtypeVariant)
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, vis: V) -> Result<V::Value> {
        if self.consume_ws_("(")? {
            let res = vis.visit_seq(self.access_tuple());
            let val = self.watch(res)?;

            if self.consume_ws_(")")? {
                Ok(val)
            } else {
                self.raise(ErrorKind::Expected("`)`"))
            }
        } else {
            self.raise(ErrorKind::ExpectedTupleVariant)
        }
    }

    fn struct_variant<V: Visitor<'de>>(self, _fields: &'static [&'static str], vis: V) -> Result<V::Value> {
        if self.consume_ws_("{")? {
            let res = vis.visit_map(self.access_struct());
            let val = self.watch(res)?;

            if self.consume_ws_("}")? {
                Ok(val)
            } else {
                self.raise(ErrorKind::Expected("`}`"))
            }
        } else {
            self.raise(ErrorKind::ExpectedStructVariant)
        }
    }
}
