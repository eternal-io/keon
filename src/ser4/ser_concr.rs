use super::{PrivateMethod, SerializerImpl};
use crate::value::{Ident, NumberSuffix};
use core::{
    fmt,
    ops::{Deref, DerefMut},
};
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
        SerializeTupleVariant,
    },
    Serialize, Serializer,
};

impl<T: ?Sized + Serialize> super::Serialize for T {
    #[expect(private_interfaces)]
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut super::Serializer<Impl>, _: PrivateMethod) -> fmt::Result {
        self.serialize(SerializerWrapper(ser))
    }
}

//==================================================================================================

/// Avoid direct use of [`super::Serializer`] as [`serde::Serializer`].
struct SerializerWrapper<'a, Impl>(&'a mut super::Serializer<Impl>);

impl<Impl> Deref for SerializerWrapper<'_, Impl> {
    type Target = super::Serializer<Impl>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<Impl> DerefMut for SerializerWrapper<'_, Impl> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<Impl: SerializerImpl> Serializer for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    #[rustfmt::skip]    fn serialize_bool  (mut self, v: bool ) -> fmt::Result { self.push_bool  (v) }
    #[rustfmt::skip]    fn serialize_char  (mut self, v: char ) -> fmt::Result { self.push_char  (v) }
    #[rustfmt::skip]    fn serialize_i8    (mut self, v: i8   ) -> fmt::Result { self.push_i8    (v) }
    #[rustfmt::skip]    fn serialize_i16   (mut self, v: i16  ) -> fmt::Result { self.push_i16   (v) }
    #[rustfmt::skip]    fn serialize_i32   (mut self, v: i32  ) -> fmt::Result { self.push_i32   (v) }
    #[rustfmt::skip]    fn serialize_i64   (mut self, v: i64  ) -> fmt::Result { self.push_i64   (v) }
    #[rustfmt::skip]    fn serialize_i128  (mut self, v: i128 ) -> fmt::Result { self.push_i128  (v) }
    #[rustfmt::skip]    fn serialize_u8    (mut self, v: u8   ) -> fmt::Result { self.push_u8    (v) }
    #[rustfmt::skip]    fn serialize_u16   (mut self, v: u16  ) -> fmt::Result { self.push_u16   (v) }
    #[rustfmt::skip]    fn serialize_u32   (mut self, v: u32  ) -> fmt::Result { self.push_u32   (v) }
    #[rustfmt::skip]    fn serialize_u64   (mut self, v: u64  ) -> fmt::Result { self.push_u64   (v) }
    #[rustfmt::skip]    fn serialize_u128  (mut self, v: u128 ) -> fmt::Result { self.push_u128  (v) }
    #[rustfmt::skip]    fn serialize_f32   (mut self, v: f32  ) -> fmt::Result { self.push_f32   (v) }
    #[rustfmt::skip]    fn serialize_f64   (mut self, v: f64  ) -> fmt::Result { self.push_f64   (v) }
    #[rustfmt::skip]    fn serialize_str   (mut self, v: &str ) -> fmt::Result { self.push_str   (v) }
    #[rustfmt::skip]    fn serialize_bytes (mut self, v: &[u8]) -> fmt::Result { self.push_bytes (v) }

    fn serialize_unit(mut self) -> fmt::Result {
        self.push_unit()
    }
    fn serialize_unit_struct(mut self, name: &'static str) -> fmt::Result {
        self.push_unit_struct(Some(Ident::new_unchecked(name)))
    }
    fn serialize_unit_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str) -> fmt::Result {
        let _ = variant_index;
        self.push_unit_variant(Some(Ident::new_unchecked(name)), Ident::new_unchecked(variant))
    }

    fn serialize_none(mut self) -> fmt::Result {
        self.push_maybe_begin()?;
        self.push_maybe_end()
    }
    fn serialize_some<T: ?Sized + Serialize>(mut self, value: &T) -> fmt::Result {
        self.push_maybe_begin()?;
        self.serialize_inner(value)?;
        self.push_maybe_end()
    }
    fn serialize_seq(mut self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let _ = len;
        self.push_array_begin()?;
        Ok(self)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(mut self, name: &'static str, value: &T) -> fmt::Result {
        self.push_newtype_begin(Some(Ident::new_unchecked(name)))?;
        self.serialize_inner(value)?;
        self.push_newtype_end()
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> fmt::Result {
        let _ = variant_index;
        self.push_tuple_variant_begin(Some(Ident::new_unchecked(name)), Ident::new_unchecked(variant))?;
        self.serialize_inner(value)?;
        self.push_tuple_like_end()
    }

    fn serialize_tuple(mut self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        let _ = len;
        self.push_tuple_begin()?;
        Ok(self)
    }
    fn serialize_tuple_struct(
        mut self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        let _ = len;
        self.push_tuple_struct_begin(Some(Ident::new_unchecked(name)))?;
        Ok(self)
    }
    fn serialize_tuple_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        let _ = len;
        let _ = variant_index;
        self.push_tuple_variant_begin(Some(Ident::new_unchecked(name)), Ident::new_unchecked(variant))?;
        Ok(self)
    }

    fn serialize_map(mut self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let _ = len;
        self.push_map_begin()?;
        Ok(self)
    }
    fn serialize_struct(mut self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        let _ = len;
        self.push_map_struct_begin(Some(Ident::new_unchecked(name)))?;
        Ok(self)
    }
    fn serialize_struct_variant(
        mut self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        let _ = len;
        let _ = variant_index;
        self.push_map_variant_begin(Some(Ident::new_unchecked(name)), Ident::new_unchecked(variant))?;
        Ok(self)
    }
}

impl<Impl: SerializerImpl> SerializeSeq for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        self.serialize_inner(value)?;
        self.push_comma()
    }
    fn end(mut self) -> fmt::Result {
        self.push_array_end()
    }
}

impl<Impl: SerializerImpl> SerializeTuple for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        self.serialize_inner(value)?;
        self.push_comma()
    }
    fn end(mut self) -> fmt::Result {
        self.push_tuple_like_end()
    }
}
impl<Impl: SerializerImpl> SerializeTupleStruct for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        self.serialize_inner(value)?;
        self.push_comma()
    }
    fn end(mut self) -> fmt::Result {
        self.push_tuple_like_end()
    }
}
impl<Impl: SerializerImpl> SerializeTupleVariant for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        self.serialize_inner(value)?;
        self.push_comma()
    }
    fn end(mut self) -> fmt::Result {
        self.push_tuple_like_end()
    }
}

impl<Impl: SerializerImpl> SerializeMap for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> fmt::Result {
        self.serialize_inner(key)?;
        self.push_fat_arrow()
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        self.serialize_inner(value)?;
        self.push_comma()
    }
    fn end(mut self) -> fmt::Result {
        self.push_map_like_end()
    }
}
impl<Impl: SerializerImpl> SerializeStruct for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, field: &'static str, value: &T) -> fmt::Result {
        self.push_identifier(Ident::new_unchecked(field))?;
        self.push_colon()?;
        self.serialize_inner(value)?;
        self.push_comma()
    }
    fn end(mut self) -> fmt::Result {
        self.push_map_like_end()
    }
}
impl<Impl: SerializerImpl> SerializeStructVariant for SerializerWrapper<'_, Impl> {
    type Ok = ();
    type Error = fmt::Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, field: &'static str, value: &T) -> fmt::Result {
        self.push_identifier(Ident::new_unchecked(field))?;
        self.push_colon()?;
        self.serialize_inner(value)?;
        self.push_comma()
    }
    fn end(mut self) -> fmt::Result {
        self.push_map_like_end()
    }
}
