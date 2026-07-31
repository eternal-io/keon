use super::*;
use crate::de4::error::{ErrorKind, ResultKind};
use serde::{de::Visitor, Deserialize, Deserializer};

impl Value2 {
    pub fn deserialize_into<'de, T: Deserialize<'de>>(&mut self) -> ResultKind<T> {
        T::deserialize(self)
    }
}

impl<'de> Deserializer<'de> for &mut Value2 {
    type Error = ErrorKind;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
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

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        todo!()
    }
}
