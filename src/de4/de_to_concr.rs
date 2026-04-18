use super::{error::*, source::*};
use core::ops::{Deref, DerefMut};
use serde::{de::Visitor, Deserializer};

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
pub(crate) struct DeserializerWrapper<'a, R>(&'a mut super::Deserializer<R>);

impl<'a, R> Deref for DeserializerWrapper<'a, R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        &self.0.src
    }
}
impl<'a, R> DerefMut for DeserializerWrapper<'a, R> {
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

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }
}
