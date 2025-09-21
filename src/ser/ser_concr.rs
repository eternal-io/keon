#![allow(unused_variables)] // TODO!

use super::*;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
        SerializeTupleVariant,
    },
    Serialize, Serializer,
};

impl<T: Serialize> Seriable for T {
    fn seria_via<W: Write>(&self, ser: &mut Serria<W>) -> SeriaResult {
        self.serialize(ser)
    }
}

//------------------------------------------------------------------------------

impl<W: Write> Serializer for &mut Serria<W> {
    type Ok = ();
    type Error = SeriaError;
    type SerializeSeq = Compound;
    type SerializeTuple = Compound;
    type SerializeTupleStruct = Compound;
    type SerializeTupleVariant = Compound;
    type SerializeMap = Compound;
    type SerializeStruct = Compound;
    type SerializeStructVariant = Compound;

    fn serialize_bool(self, v: bool) -> SeriaResult {
        todo!()
    }

    fn serialize_i8(self, v: i8) -> SeriaResult {
        todo!()
    }
    fn serialize_i16(self, v: i16) -> SeriaResult {
        todo!()
    }
    fn serialize_i32(self, v: i32) -> SeriaResult {
        todo!()
    }
    fn serialize_i64(self, v: i64) -> SeriaResult {
        todo!()
    }
    fn serialize_i128(self, v: i128) -> SeriaResult {
        todo!()
    }

    fn serialize_u8(self, v: u8) -> SeriaResult {
        todo!()
    }
    fn serialize_u16(self, v: u16) -> SeriaResult {
        todo!()
    }
    fn serialize_u32(self, v: u32) -> SeriaResult {
        todo!()
    }
    fn serialize_u64(self, v: u64) -> SeriaResult {
        todo!()
    }
    fn serialize_u128(self, v: u128) -> SeriaResult {
        todo!()
    }

    fn serialize_f32(self, v: f32) -> SeriaResult {
        todo!()
    }
    fn serialize_f64(self, v: f64) -> SeriaResult {
        todo!()
    }

    fn serialize_char(self, v: char) -> SeriaResult {
        todo!()
    }
    fn serialize_str(self, v: &str) -> SeriaResult {
        todo!()
    }
    fn serialize_bytes(self, v: &[u8]) -> SeriaResult {
        todo!()
    }

    fn serialize_none(self) -> SeriaResult {
        todo!()
    }
    fn serialize_some<T>(self, value: &T) -> SeriaResult
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> SeriaResult {
        todo!()
    }
    fn serialize_unit_struct(self, name: &'static str) -> SeriaResult {
        todo!()
    }
    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> SeriaResult {
        todo!()
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> SeriaResult
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }
    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> SeriaResult
    where
        T: ?Sized + serde::Serialize,
    {
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

//------------------------------------------------------------------------------

#[doc(hidden)]
pub struct Compound;

impl SerializeSeq for Compound {
    type Ok = ();
    type Error = SeriaError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> SeriaResult {
        todo!()
    }
}

impl SerializeTuple for Compound {
    type Ok = ();
    type Error = SeriaError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> SeriaResult {
        todo!()
    }
}

impl SerializeTupleStruct for Compound {
    type Ok = ();
    type Error = SeriaError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> SeriaResult {
        todo!()
    }
}

impl SerializeTupleVariant for Compound {
    type Ok = ();
    type Error = SeriaError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> SeriaResult {
        todo!()
    }
}

impl SerializeMap for Compound {
    type Ok = ();
    type Error = SeriaError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> SeriaResult {
        todo!()
    }
}

impl SerializeStruct for Compound {
    type Ok = ();
    type Error = SeriaError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> SeriaResult {
        todo!()
    }
}

impl SerializeStructVariant for Compound {
    type Ok = ();
    type Error = SeriaError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> SeriaResult {
        todo!()
    }
}
