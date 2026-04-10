use super::{private::*, SerializerImpl};
use core::fmt;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
        SerializeTupleVariant,
    },
    Serialize, Serializer,
};

impl<T: Serialize> super::Serialize for T {
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut super::Serializer<Impl>) -> fmt::Result {
        self.serialize(ser)
    }
}

//==================================================================================================

impl<Impl: SerializerImpl> Serializer for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    #[rustfmt::skip]    fn serialize_bool  (self, v: bool ) -> fmt::Result { self.0.push(Token::Literal(Literal::Bool(v)))          }
    #[rustfmt::skip]    fn serialize_char  (self, v: char ) -> fmt::Result { self.0.push(Token::Literal(Literal::Char(v)))          }
    #[rustfmt::skip]    fn serialize_i8    (self, v: i8   ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_i16   (self, v: i16  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_i32   (self, v: i32  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_i64   (self, v: i64  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_i128  (self, v: i128 ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_u8    (self, v: u8   ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_u16   (self, v: u16  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_u32   (self, v: u32  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_u64   (self, v: u64  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_u128  (self, v: u128 ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_f32   (self, v: f32  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_f64   (self, v: f64  ) -> fmt::Result { self.0.push(Token::Literal(Literal::Number(v.into()))) }
    #[rustfmt::skip]    fn serialize_str   (self, v: &str ) -> fmt::Result { self.0.push(Token::Literal(Literal::Str(v)))           }
    #[rustfmt::skip]    fn serialize_bytes (self, v: &[u8]) -> fmt::Result { self.0.push(Token::Literal(Literal::Bytes(v)))         }

    fn serialize_none(self) -> fmt::Result {
        todo!()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> fmt::Result {
        todo!()
    }

    fn serialize_unit(self) -> fmt::Result {
        todo!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> fmt::Result {
        todo!()
    }

    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> fmt::Result {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, name: &'static str, value: &T) -> fmt::Result {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> fmt::Result {
        todo!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        todo!()
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!()
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        todo!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}

impl<Impl: SerializerImpl> SerializeSeq for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        todo!()
    }
    fn end(self) -> fmt::Result {
        todo!()
    }
}

impl<Impl: SerializerImpl> SerializeTuple for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        todo!()
    }
    fn end(self) -> fmt::Result {
        todo!()
    }
}

impl<Impl: SerializerImpl> SerializeTupleStruct for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        todo!()
    }
    fn end(self) -> fmt::Result {
        todo!()
    }
}

impl<Impl: SerializerImpl> SerializeTupleVariant for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        todo!()
    }
    fn end(self) -> fmt::Result {
        todo!()
    }
}

impl<Impl: SerializerImpl> SerializeMap for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> fmt::Result {
        todo!()
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        todo!()
    }
    fn end(self) -> fmt::Result {
        todo!()
    }
}

impl<Impl: SerializerImpl> SerializeStruct for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> fmt::Result {
        todo!()
    }
    fn end(self) -> fmt::Result {
        todo!()
    }
}

impl<Impl: SerializerImpl> SerializeStructVariant for &mut super::Serializer<Impl> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> fmt::Result {
        todo!()
    }
    fn end(self) -> fmt::Result {
        todo!()
    }
}
