use super::*;
use crate::de4::error::{ErrorImpl, Result, ResultKind};
use serde::{de::Visitor, forward_to_deserialize_any, Deserialize, Deserializer};

impl Number2 {
    pub fn deserialize_to<'de, T: Deserialize<'de>>(&self) -> Result<T> {
        Ok(T::deserialize(self.deserializer())?)
    }

    pub(crate) fn deserializer(&self) -> NumberWrapper<'_> {
        NumberWrapper(self)
    }
}

pub(crate) struct NumberWrapper<'a>(&'a Number2);

impl<'de> Deserializer<'de> for NumberWrapper<'_> {
    type Error = ErrorImpl;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        match *self.0 {
            Number2::Int8(v) => visitor.visit_i8(v),
            Number2::Int16(v) => visitor.visit_i16(v),
            Number2::Int32(v) => visitor.visit_i32(v),
            Number2::Int64(v) => visitor.visit_i64(v),
            Number2::Int128 { lo, hi } => visitor.visit_i128((hi as i128) << 64 | lo as i128),
            Number2::UInt8(v) => visitor.visit_u8(v),
            Number2::UInt16(v) => visitor.visit_u16(v),
            Number2::UInt32(v) => visitor.visit_u32(v),
            Number2::UInt64(v) => visitor.visit_u64(v),
            Number2::UInt128 { lo, hi } => visitor.visit_u128((hi as u128) << 64 | lo as u128),
            Number2::Float32(Float32(v)) => visitor.visit_f32(v),
            Number2::Float64(Float64(v)) => visitor.visit_f64(v),
            Number2::IntNoSuffix(v) => visitor.visit_i64(v),
            Number2::UIntNoSuffix(v) => visitor.visit_u64(v),
            Number2::FloatNoSuffix(Float64(v)) => visitor.visit_f64(v),
        }
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        visitor.visit_none()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier
    }
}

//------------------------------------------------------------------------------

impl Scalar {
    pub fn deserialize_to<'de, T: Deserialize<'de>>(&self) -> Result<T> {
        Ok(T::deserialize(self.deserializer())?)
    }

    pub(crate) fn deserializer(&self) -> ScalarWrapper<'_> {
        ScalarWrapper(self)
    }
}

pub(crate) struct ScalarWrapper<'a>(&'a Scalar);

impl<'de> Deserializer<'de> for ScalarWrapper<'_> {
    type Error = ErrorImpl;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        match self.0 {
            Scalar::Char(ch) => visitor.visit_char(*ch),
            Scalar::Number(num) => num.deserializer().deserialize_any(visitor),
        }
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        visitor.visit_none()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier
    }
}
