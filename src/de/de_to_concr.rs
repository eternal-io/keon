use super::*;
use core::ops::{Deref, DerefMut};
use serde::{
    de::{value::BorrowedStrDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

impl<'de, T: Deserialize<'de>> Parsable<'de> for T {
    #[inline]
    fn parse_limited_via(der: &mut Parser<'de>, _limit: Option<u32>) -> Result<Self> {
        Self::deserialize(der)
    }
}

impl<'de> Parser<'de> {
    #[inline]
    fn watch_at<T>(&mut self, pos: usize, mut res: Result<T>) -> Result<T> {
        if let Err(e) = &mut res {
            self.corrupted = true;
            e.pos = pos;
        }
        res
    }

    #[inline]
    fn deserialize_guard(&mut self) -> Result<()> {
        if self.corrupted {
            self.raise(ErrorKind::Corrupted)
        } else {
            self.consume_whitespace_comment_first()
        }
    }

    fn consume_nominal_path_of_struct(&mut self, name: &'static str) -> Result<bool> {
        let res = match self.consume_ident_or_underscore()? {
            // _ //
            None => true,
            // Name //
            Some(ident) => match self.consume_ws_("::")? {
                // Name //
                false => name == ident,
                // Name::ActualName //
                true => name == self.consume_ident()?,
            },
        };

        Ok(res)
    }

    fn consume_nominal_path_of_enum(&mut self, name: &'static str) -> Result<Option<&'de str>> {
        let res = match self.consume_ident_or_underscore()? {
            // _ //
            None => match self.consume_ws_("::")? {
                // _ //
                false => None,
                // _::Variant //
                true => Some(self.consume_ident()?),
            },
            // Name //
            Some(ident) => match self.consume_ws_("::")? {
                // Name //
                false => Some(ident),
                // Name::Variant //
                true => match name == ident {
                    false => None,
                    true => Some(self.consume_ident()?),
                },
            },
        };

        Ok(res)
    }
}

//------------------------------------------------------------------------------

macro_rules! deserialize_integer {
    ( $self:ident, $parse_fn:ident, $ty:ident, $visitor:ident, $visit_fn:ident ) => {{
        $self.deserialize_guard()?;
        $self.consume_ws_("long")?;

        let start = $self.pos;
        let num = $self.$parse_fn::<$ty>()?;
        let val = $self.watch_at(start, $visitor.$visit_fn(num))?;

        Ok(val)
    }};
}

macro_rules! deserialize_float {
    ( $self:ident, $ty:ident, $visitor:ident, $visit_fn:ident ) => {{
        $self.deserialize_guard()?;

        let start = $self.pos;
        let num = $self.parse_float::<$ty>()?;
        let val = $self.watch_at(start, $visitor.$visit_fn(num))?;

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
        self.deserialize_guard()?;

        let start = self.pos;

        if self.consume_ws_("true")? {
            self.watch_at(start, vis.visit_bool(true))
        } else if self.consume_ws_("false")? {
            self.watch_at(start, vis.visit_bool(false))
        } else {
            self.raise(ErrorKind::ExpectedBoolean)
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;
        let res = vis.visit_char(self.parse_char()?);

        self.watch_at(start, res)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if let Some(b'b') = self.peek_byte() {
            self.deserialize_guard()?;

            let start = self.pos;
            let res = vis.visit_u8(self.parse_byte()?);

            self.watch_at(start, res)
        } else {
            deserialize_integer!(self, parse_integer_unsigned, u8, vis, visit_u8)
        }
    }
    fn deserialize_u16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_unsigned, u16, vis, visit_u16)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_unsigned, u32, vis, visit_u32)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_unsigned, u64, vis, visit_u64)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_unsigned, u128, vis, visit_u128)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_signed, u8, vis, visit_i8)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_signed, u16, vis, visit_i16)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_signed, u32, vis, visit_i32)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_signed, u64, vis, visit_i64)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_integer!(self, parse_integer_signed, u128, vis, visit_i128)
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
        self.deserialize_guard()?;

        let start = self.pos;

        match self.parse_string_or_paragraph()? {
            Either::Left(s) => self.watch_at(start, vis.visit_borrowed_str(s)),
            Either::Right(buf) => self.watch_at(start, vis.visit_string(buf)),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_bytes(vis)
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;

        match self.parse_byte_string()? {
            Either::Left(bytes) => self.watch_at(start, vis.visit_borrowed_bytes(bytes)),
            Either::Right(buf) => self.watch_at(start, vis.visit_byte_buf(buf)),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;

        if self.consume_ws_("?")? {
            if self.adjacent_to_delim() {
                self.watch_at(start, vis.visit_none())
            } else {
                let res = vis.visit_some(&mut *self);
                self.watch_at(start, res)
            }
        } else {
            self.raise(ErrorKind::ExpectedMaybe)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;

        if self.consume_ws_("(")? && self.consume_ws_(")")? {
            self.watch_at(start, vis.visit_unit())
        } else {
            self.raise_at(start, ErrorKind::ExpectedUnit)
        }
    }

    //------------------------------------------------------------------------------

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        if self.consume_ws_("(")? {
            let start = self.pos;
            let res = vis.visit_seq(self.access_tuple());
            let val = self.watch_at(start, res)?;

            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::ExpectedParenClose);
            }

            Ok(val)
        } else {
            self.raise(ErrorKind::ExpectedTuple)
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        if self.consume_ws_("[")? {
            let start = self.pos;
            let res = vis.visit_seq(self.access_seq());
            let val = self.watch_at(start, res)?;

            if !self.consume_ws_("]")? {
                return self.raise(ErrorKind::ExpectedBrackClose);
            }

            Ok(val)
        } else {
            self.raise(ErrorKind::ExpectedSequence)
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        if self.consume_ws_("{")? {
            let start = self.pos;
            let res = vis.visit_map(self.access_map());
            let val = self.watch_at(start, res)?;

            if !self.consume_ws_("}")? {
                return self.raise(ErrorKind::ExpectedBraceClose);
            }

            Ok(val)
        } else {
            self.raise(ErrorKind::ExpectedMap)
        }
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;

        if self.consume_nominal_path_of_struct(name)? {
            /* Name */
            if self.consume_ws_("(")? {
                /* Name () */
                if !self.consume_ws_(")")? {
                    return self.raise(ErrorKind::ExpectedParenClose);
                }
            }
        } else if self.consume_ws_("(")? {
            /* () */
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::ExpectedParenClose);
            }
        } else {
            return self.raise(ErrorKind::ExpectedUnitStruct { name });
        }

        self.watch_at(start, vis.visit_unit())
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;

        if self.consume_nominal_path_of_struct(name)? && self.consume_ws_("(")? {
            let res = vis.visit_newtype_struct(&mut *self);
            let val = self.watch_at(start, res)?;

            self.consume_ws_(",")?;
            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::ExpectedParenClose);
            }

            Ok(val)
        } else {
            self.raise_at(start, ErrorKind::ExpectedNewtypeStruct { name })
        }
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, _len: usize, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;

        if self.consume_nominal_path_of_struct(name)? && self.consume_ws_("(")? {
            let res = vis.visit_seq(self.access_tuple());
            let val = self.watch_at(start, res)?;

            if !self.consume_ws_(")")? {
                return self.raise(ErrorKind::ExpectedParenClose);
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
        self.deserialize_guard()?;

        let start = self.pos;

        if self.consume_nominal_path_of_struct(name)? && self.consume_ws_("{")? {
            let res = vis.visit_map(self.access_struct());
            let val = self.watch_at(start, res)?;

            if !self.consume_ws_("}")? {
                return self.raise(ErrorKind::ExpectedBraceClose);
            }

            Ok(val)
        } else {
            self.raise_at(start, ErrorKind::ExpectedStruct { name })
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;
        let res = vis.visit_borrowed_str(self.consume_ident()?);

        self.watch_at(start, res)
    }

    //------------------------------------------------------------------------------

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        self.deserialize_guard()?;

        let start = self.pos;
        let Some(variant_name) = self.consume_nominal_path_of_enum(name)? else {
            return self.raise_at(start, ErrorKind::ExpectedEnum { name });
        };

        if !variants.contains(&variant_name) {
            return self.raise_at(start, ErrorKind::ExpectedVariant { variants });
        }

        let res = vis.visit_enum(self.access_enum(variant_name));
        let val = self.watch_at(start, res)?;

        Ok(val)
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
                    return der.raise(ErrorKind::ExpectedColon);
                }
            }
            false => {
                if !der.consume_ws_("=>")? {
                    return der.raise(ErrorKind::ExpectedFatArrow);
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
    type Variant = VariantAccessor<'a, 'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        Ok((
            // NOTE:
            // This line of code does not call `deserialize_identifier`, because we have
            // complex nominal paths, and variant names have been extracted separately.
            seed.deserialize(BorrowedStrDeserializer::<Error>::new(self.variant_name))?,
            VariantAccessor(self.der),
        ))
    }
}

//------------------------------------------------------------------------------

struct VariantAccessor<'a, 'de>(&'a mut Parser<'de>);

impl<'a, 'de> Deref for VariantAccessor<'a, 'de> {
    type Target = Parser<'de>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, 'de> DerefMut for VariantAccessor<'a, 'de> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a, 'de> VariantAccess<'de> for VariantAccessor<'a, 'de> {
    type Error = Error;

    fn unit_variant(mut self) -> Result<()> {
        if self.adjacent_to_delim() {
            Ok(())
        } else {
            self.raise(ErrorKind::ExpectedUnitVariant)
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(mut self, seed: T) -> Result<T::Value> {
        if self.consume_ws_("(")? {
            let val = seed.deserialize(&mut *self.0)?;

            self.consume_ws_(",")?;
            if self.consume_ws_(")")? {
                Ok(val)
            } else {
                self.raise(ErrorKind::ExpectedParenClose)
            }
        } else {
            self.raise(ErrorKind::ExpectedNewtypeVariant)
        }
    }

    fn tuple_variant<V: Visitor<'de>>(mut self, _len: usize, vis: V) -> Result<V::Value> {
        let start = self.pos;

        if self.consume_ws_("(")? {
            let res = vis.visit_seq(self.access_tuple());
            let val = self.watch_at(start, res)?;

            if self.consume_ws_(")")? {
                Ok(val)
            } else {
                self.raise(ErrorKind::ExpectedParenClose)
            }
        } else {
            self.raise(ErrorKind::ExpectedTupleVariant)
        }
    }

    fn struct_variant<V: Visitor<'de>>(mut self, _fields: &'static [&'static str], vis: V) -> Result<V::Value> {
        let start = self.pos;

        if self.consume_ws_("{")? {
            let res = vis.visit_map(self.access_struct());
            let val = self.watch_at(start, res)?;

            if self.consume_ws_("}")? {
                Ok(val)
            } else {
                self.raise(ErrorKind::ExpectedBraceClose)
            }
        } else {
            self.raise(ErrorKind::ExpectedStructVariant)
        }
    }
}
