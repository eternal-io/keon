use super::{error::*, source::*};
use crate::value::NominalPathRef;
use core::ops::{Deref, DerefMut};
use either::Either;
use serde::{
    de::{value::StrDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    Deserialize, Deserializer,
};

impl<'de, T: Deserialize<'de>> super::Deserialize<'de> for T {
    fn deserialize_with<R: Source<'de>>(der: &mut super::Deserializer<R>) -> ResultKind<Self> {
        T::deserialize(DeserializerWrapper(der))
    }
}

//==================================================================================================

macro_rules! deserialize_number {
    ( $method:ident, $parsing:ident, $visiting:ident ) => {
        fn $method<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
            if let NumberKind::Byte = self.begin_number()? {
                visitor.visit_u8(self.parse_byte()?)
            } else {
                visitor.$visiting(self.$parsing()?)
            }
        }
    };
}

/// Avoid direct use [`super::Deserializer`] as [`serde::Deserializer`] that bypass error location fix.
#[repr(transparent)]
struct DeserializerWrapper<'a, R>(&'a mut super::Deserializer<R>);

impl<R> DeserializerWrapper<'_, R> {
    fn reborrow<'a>(&'a mut self) -> DeserializerWrapper<'a, R> {
        DeserializerWrapper(self.0)
    }

    fn ttl_enter(&mut self) -> ResultKind {
        self.0.ttl_enter()
    }

    fn ttl_leave(&mut self) {
        self.0.ttl_leave()
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

    fn deserialize_unit<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.begin_unit()?;
        visitor.visit_unit()
    }

    fn deserialize_bool<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        visitor.visit_bool(self.begin_bool()?)
    }

    deserialize_number!(deserialize_i8, parse_i8, visit_i8);
    deserialize_number!(deserialize_i16, parse_i16, visit_i16);
    deserialize_number!(deserialize_i32, parse_i32, visit_i32);
    deserialize_number!(deserialize_i64, parse_i64, visit_i64);
    deserialize_number!(deserialize_i128, parse_i128, visit_i128);
    deserialize_number!(deserialize_u8, parse_u8, visit_u8);
    deserialize_number!(deserialize_u16, parse_u16, visit_u16);
    deserialize_number!(deserialize_u32, parse_u32, visit_u32);
    deserialize_number!(deserialize_u64, parse_u64, visit_u64);
    deserialize_number!(deserialize_u128, parse_u128, visit_u128);
    deserialize_number!(deserialize_f32, parse_f32, visit_f32);
    deserialize_number!(deserialize_f64, parse_f64, visit_f64);

    fn deserialize_char<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.begin_char()?;
        visitor.visit_char(self.parse_char()?)
    }

    fn deserialize_str<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        let kind = self.begin_string()?;
        match self.0.src.parse_string(kind, &mut self.0.buf)? {
            Either::Left(ref_de) => visitor.visit_borrowed_str(ref_de),
            Either::Right(ref_tmp) => visitor.visit_str(ref_tmp),
        }
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        let kind = self.begin_bytes()?;
        match self.0.src.parse_bytes(kind, &mut self.0.buf)? {
            Either::Left(ref_de) => visitor.visit_borrowed_bytes(ref_de),
            Either::Right(ref_tmp) => visitor.visit_bytes(ref_tmp),
        }
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.begin_maybe()?;
        if self.seek_delim()?.is_none() {
            self.ttl_enter()?;
            let val = visitor.visit_some(self.reborrow())?;
            self.ttl_leave();
            Ok(val)
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(mut self, len: usize, visitor: V) -> ResultKind<V::Value> {
        let _ = len;
        self.ttl_enter()?;
        self.begin_tuple()?;
        let val = visitor.visit_seq(self.reborrow())?;
        self.seek_delim_expected()?.expect(PunctDelim::Paren)?;
        self.eat_delim();
        self.ttl_leave();
        Ok(val)
    }
    fn deserialize_seq<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.ttl_enter()?;
        self.begin_sequence()?;
        let val = visitor.visit_seq(self.reborrow())?;
        self.seek_delim_expected()?.expect(PunctDelim::Brack)?;
        self.eat_delim();
        self.ttl_leave();
        Ok(val)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_map_like::<V, false>(visitor)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(mut self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_struct_name(name)?;
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(mut self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_struct_name(name)?;
        self.ttl_enter()?;
        let val = visitor.visit_newtype_struct(self.reborrow())?;
        self.ttl_leave();
        Ok(val)
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
        self.deserialize_struct_body(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        let _ = variants;
        visitor.visit_enum(EnumAccessor(name, self))
    }

    // NOTE: This method is called when deserialize struct field name.
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        visitor.visit_str(self.0.src.begin_identifier(&mut self.0.buf)?)
    }
}

impl<'de, R: Source<'de>> DeserializerWrapper<'_, R> {
    #[inline]
    fn deserialize_struct_name(&mut self, name: &'static str) -> ResultKind {
        if let NominalPathRef::Single { name: name_parsed } | NominalPathRef::Dual { name: name_parsed, .. } =
            self.0.src.begin_nominal(&mut self.0.buf)?
        {
            if name_parsed != name {
                return Err(ErrorKind::ExpectedAnotherStruct { name });
            }
        }
        Ok(())
    }

    #[inline]
    fn deserialize_struct_body<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_map_like::<V, true>(visitor)
    }

    #[inline]
    fn deserialize_map_like<V: Visitor<'de>, const STRUCT_MODE: bool>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.ttl_enter()?;
        self.begin_map()?;
        let val = visitor.visit_map(MapAccessor::<R, STRUCT_MODE>(self.reborrow()))?;
        self.seek_delim_expected()?.expect(PunctDelim::Brace)?;
        self.eat_delim();
        self.ttl_leave();
        Ok(val)
    }
}

//==================================================================================================

impl<'de, R: Source<'de>> SeqAccess<'de> for DeserializerWrapper<'_, R> {
    type Error = ErrorKind;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> ResultKind<Option<T::Value>> {
        if let Some(PunctDelim::Brack) = self.seek_delim()? {
            return Ok(None);
        }

        let val = seed.deserialize(self.reborrow())?;

        if let PunctDelim::Comma = self.seek_delim_expected()? {
            self.eat_delim();
        }

        Ok(Some(val))
    }
}

struct MapAccessor<'a, R, const STRUCT_MODE: bool>(DeserializerWrapper<'a, R>);

impl<'a, 'de, R: Source<'de>, const STRUCT_MODE: bool> MapAccess<'de> for MapAccessor<'_, R, STRUCT_MODE> {
    type Error = ErrorKind;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        if let Some(PunctDelim::Brace) = self.0.seek_delim()? {
            return Ok(None);
        }

        let val = seed.deserialize(self.0.reborrow())?;

        if STRUCT_MODE {
            self.0.seek_delim_expected()?.expect(PunctDelim::Colon)?;
        } else {
            self.0.seek_delim_expected()?.expect(PunctDelim::FatArrow)?;
        }
        self.0.eat_delim();

        Ok(Some(val))
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        let val = seed.deserialize(self.0.reborrow())?;

        if let PunctDelim::Comma = self.0.seek_delim_expected()? {
            self.0.eat_delim();
        }

        Ok(val)
    }
}

struct EnumAccessor<'a, R>(&'static str, DeserializerWrapper<'a, R>);

impl<'a, 'de, R: Source<'de>> EnumAccess<'de> for EnumAccessor<'a, R> {
    type Error = ErrorKind;
    type Variant = DeserializerWrapper<'a, R>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> ResultKind<(V::Value, Self::Variant)> {
        let EnumAccessor(parent, der) = self;
        let name = match der.0.src.begin_nominal(&mut der.0.buf)? {
            NominalPathRef::Underscore => return Err(ErrorKind::ExpectedVariantName),
            NominalPathRef::Single { name } => name,
            NominalPathRef::Dual {
                name,
                parent: parent_parsed,
            } => {
                if parent_parsed != parent {
                    return Err(ErrorKind::ExpectedAnotherEnum { name: parent });
                }
                name
            }
        };

        Ok((seed.deserialize(StrDeserializer::<ErrorKind>::new(name))?, der))
    }
}

impl<'de, R: Source<'de>> VariantAccess<'de> for DeserializerWrapper<'_, R> {
    type Error = ErrorKind;

    fn unit_variant(mut self) -> ResultKind<()> {
        self.seek_delim_expected()?;
        Ok(())
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(mut self, seed: T) -> ResultKind<T::Value> {
        self.ttl_enter()?;
        let val = seed.deserialize(self.reborrow())?;
        self.ttl_leave();
        Ok(val)
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_tuple(len, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> ResultKind<V::Value> {
        let _ = fields;
        self.deserialize_struct_body(visitor)
    }
}
