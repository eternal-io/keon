use super::{error::*, source::*, PrivateMethod};
use crate::value::NominalPathRef;
use core::ops::{Deref, DerefMut};
use either::Either;
use serde::{
    de::{value::StrDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

impl<'de, T: Deserialize<'de>> super::Deserialize<'de> for T {
    #[expect(private_interfaces)]
    fn deserialize_with<R: Source<'de>>(der: &mut super::Deserializer<R>, _: PrivateMethod) -> ResultKind<Self> {
        T::deserialize(DeserializerWrapper(der))
    }
}

//==================================================================================================

macro_rules! recursion_guard {
    ($self:ident, $expr:expr) => {{
        $self.0.ttl_enter()?;
        let res = $expr;
        $self.0.ttl_leave();
        res
    }};
}

macro_rules! deserialize_integer {
    ($method:ident, $visiting:ident, $parsing:ident) => {
        fn $method<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
            self.eat_ws()?;
            if self.try_byte()? {
                visitor.visit_u8(self.parse_byte()?)
            } else {
                visitor.$visiting(self.$parsing()?)
            }
        }
    };
}

macro_rules! deserialize_float {
    ($method:ident, $visiting:ident, $parsing:ident) => {
        fn $method<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
            self.eat_ws()?;
            visitor.$visiting(self.$parsing()?)
        }
    };
}

/// Avoid direct use of [`super::Deserializer`] as [`serde::Deserializer`] that bypasses error location fix.
struct DeserializerWrapper<'a, R>(&'a mut super::Deserializer<R>);

impl<R> DeserializerWrapper<'_, R> {
    fn reborrow<'a>(&'a mut self) -> DeserializerWrapper<'a, R> {
        DeserializerWrapper(self.0)
    }
}

impl<R> Deref for DeserializerWrapper<'_, R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        &self.0.src
    }
}

impl<R> DerefMut for DeserializerWrapper<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.src
    }
}

impl<'de, R: Source<'de>> Deserializer<'de> for DeserializerWrapper<'_, R> {
    type Error = ErrorKind;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        let _ = visitor;
        Err(ErrorKind::WontImplement)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        let _ = visitor;
        Err(ErrorKind::WontImplement)
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.parse_unit()?;
        visitor.visit_unit()
    }

    fn deserialize_bool<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.eat_ws()?;
        visitor.visit_bool(self.parse_bool()?)
    }

    deserialize_integer!(deserialize_i8, visit_i8, parse_i8);
    deserialize_integer!(deserialize_i16, visit_i16, parse_i16);
    deserialize_integer!(deserialize_i32, visit_i32, parse_i32);
    deserialize_integer!(deserialize_i64, visit_i64, parse_i64);
    deserialize_integer!(deserialize_i128, visit_i128, parse_i128);

    deserialize_integer!(deserialize_u8, visit_u8, parse_u8);
    deserialize_integer!(deserialize_u16, visit_u16, parse_u16);
    deserialize_integer!(deserialize_u32, visit_u32, parse_u32);
    deserialize_integer!(deserialize_u64, visit_u64, parse_u64);
    deserialize_integer!(deserialize_u128, visit_u128, parse_u128);

    deserialize_float!(deserialize_f32, visit_f32, parse_f32);
    deserialize_float!(deserialize_f64, visit_f64, parse_f64);

    fn deserialize_char<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.begin_char()?;
        visitor.visit_char(self.parse_char()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_str(visitor)
    }
    fn deserialize_str<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        let kind = self.begin_string()?;
        match self.0.src.parse_string(kind, &mut self.0.buf)? {
            Either::Left(ref_de) => visitor.visit_borrowed_str(ref_de),
            Either::Right(ref_tmp) => visitor.visit_str(ref_tmp),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_bytes(visitor)
    }
    fn deserialize_bytes<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        let kind = self.begin_bytes()?;
        match self.0.src.parse_bytes(kind, &mut self.0.buf)? {
            Either::Left(ref_de) => visitor.visit_borrowed_bytes(ref_de),
            Either::Right(ref_tmp) => visitor.visit_bytes(ref_tmp),
        }
    }

    //------------------------------------------------------------------------------

    fn deserialize_option<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.begin_maybe()?;
        if !self.adjacent_to_delim()? {
            recursion_guard!(self, Ok(visitor.visit_some(self.reborrow())?))
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        recursion_guard!(self, {
            self.begin_array()?;
            let val = visitor.visit_seq(self.reborrow())?;
            self.end_array()?;
            Ok(val)
        })
    }

    fn deserialize_tuple<V: Visitor<'de>>(mut self, len: usize, visitor: V) -> ResultKind<V::Value> {
        let _ = len;
        recursion_guard!(self, {
            self.begin_tuple()?;
            let val = visitor.visit_seq(self.reborrow())?;
            self.end_tuple()?;
            Ok(val)
        })
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_map_like::<V, false>(visitor)
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit_struct<V: Visitor<'de>>(mut self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_struct_name(name)?;
        self.adjacent_to_delim_expected(ErrorKind::InvalidUnit)?;
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(mut self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_struct_name(name)?;
        recursion_guard!(self, {
            self.begin_tuple()?;
            let val = visitor.visit_newtype_struct(self.reborrow())?;
            self.end_tuple()?;
            Ok(val)
        })
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        mut self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> ResultKind<V::Value> {
        self.deserialize_struct_name(name)?;
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        let _ = fields;
        self.deserialize_struct_name(name)?;
        self.deserialize_map_like::<V, true>(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        let _ = variants;
        visitor.visit_enum(EnumAccessor { name, der: self })
    }

    // NOTE: This method is called when deserialize struct field name.
    fn deserialize_identifier<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.eat_ws()?;
        visitor.visit_str(&self.0.src.parse_identifier(&mut self.0.buf)?)
    }
}

impl<'de, R: Source<'de>> DeserializerWrapper<'_, R> {
    #[inline]
    fn deserialize_struct_name(&mut self, name: &'static str) -> ResultKind {
        self.eat_ws()?;
        match self.0.src.parse_nominal_path(&mut self.0.buf)? {
            NominalPathRef::Underscore => (),
            NominalPathRef::Single { name: name_parsed } => {
                if *name_parsed != *name {
                    return Err(ErrorKind::ExpectedDifferentStructName {
                        expected: name,
                        found: name_parsed.to_string(),
                    });
                }
            }
            NominalPathRef::Dual { .. } => return Err(ErrorKind::UnexpectedPathAsStructName),
        }
        Ok(())
    }

    #[inline]
    fn deserialize_map_like<V: Visitor<'de>, const STRUCT_MODE: bool>(mut self, visitor: V) -> ResultKind<V::Value> {
        recursion_guard!(self, {
            self.begin_map_like()?;
            let val = visitor.visit_map(MapAccessor::<R, STRUCT_MODE> { der: self.reborrow() })?;
            self.end_map_like()?;
            Ok(val)
        })
    }
}

//==================================================================================================

impl<'de, R: Source<'de>> SeqAccess<'de> for DeserializerWrapper<'_, R> {
    type Error = ErrorKind;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> ResultKind<Option<T::Value>> {
        if self.adjacent_to_delim()? {
            return Ok(None);
        }
        let val = seed.deserialize(self.reborrow())?;
        self.delim(Delimiter::Comma)?;
        Ok(Some(val))
    }
}

struct MapAccessor<'a, R, const STRUCT_MODE: bool> {
    der: DeserializerWrapper<'a, R>,
}

impl<'de, R: Source<'de>, const STRUCT_MODE: bool> MapAccess<'de> for MapAccessor<'_, R, STRUCT_MODE> {
    type Error = ErrorKind;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        let MapAccessor { der } = self;
        if der.adjacent_to_delim()? {
            return Ok(None);
        }
        let val = seed.deserialize(der.reborrow())?;
        if STRUCT_MODE {
            der.delim_expected(Delimiter::Colon, ErrorKind::ExpectedColon)?;
        } else {
            der.delim_expected(Delimiter::FatArrow, ErrorKind::ExpectedFatArrow)?;
        }
        Ok(Some(val))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        let MapAccessor { der } = self;
        let val = seed.deserialize(der.reborrow())?;
        der.delim(Delimiter::Comma)?;
        Ok(val)
    }
}

struct EnumAccessor<'a, R> {
    der: DeserializerWrapper<'a, R>,
    name: &'static str,
}

impl<'a, 'de, R: Source<'de>> EnumAccess<'de> for EnumAccessor<'a, R> {
    type Error = ErrorKind;
    type Variant = DeserializerWrapper<'a, R>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> ResultKind<(V::Value, Self::Variant)> {
        let EnumAccessor { mut der, name } = self;
        der.eat_ws()?;
        let variant = match der.0.src.parse_nominal_path(&mut der.0.buf)? {
            NominalPathRef::Underscore => return Err(ErrorKind::ExpectedVariantName),
            NominalPathRef::Single { name: variant } => variant,
            NominalPathRef::Dual {
                name: variant,
                parent: name_parsed,
            } => {
                if *name_parsed != *name {
                    return Err(ErrorKind::ExpectedDifferentEnumName {
                        expected: name,
                        found: name_parsed.to_string(),
                    });
                }
                variant
            }
        };
        Ok((seed.deserialize(StrDeserializer::<ErrorKind>::new(&variant))?, der))
    }
}

impl<'de, R: Source<'de>> VariantAccess<'de> for DeserializerWrapper<'_, R> {
    type Error = ErrorKind;

    fn unit_variant(mut self) -> ResultKind<()> {
        self.adjacent_to_delim_expected(ErrorKind::InvalidUnit)?;
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(mut self, seed: T) -> ResultKind<T::Value> {
        recursion_guard!(self, {
            self.begin_tuple()?;
            let val = seed.deserialize(self.reborrow())?;
            self.end_tuple()?;
            Ok(val)
        })
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> ResultKind<V::Value> {
        let _ = fields;
        self.deserialize_map_like::<V, true>(visitor)
    }
}
