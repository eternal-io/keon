use super::*;
use crate::de4::error::{ErrorImpl, Result, ResultKind};
use alloc::collections::btree_map;
use serde::{
    de::{
        value::StrDeserializer, DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, Unexpected, VariantAccess,
        Visitor,
    },
    forward_to_deserialize_any, Deserialize, Deserializer,
};

impl Value2 {
    pub fn deserialize_to<'de, T: Deserialize<'de>>(&self) -> Result<T> {
        Ok(T::deserialize(ValueWrapper(&self))?)
    }

    fn deserializer(&self) -> ValueWrapper<'_> {
        ValueWrapper(self)
    }

    fn unexpected(&self) -> Unexpected<'static> {
        match self {
            Value2::Bool(_) => Unexpected::Other("boolean"),
            Value2::Char(_) => Unexpected::Other("character"),
            Value2::Number(_) => Unexpected::Other("number"),
            Value2::String(_) => Unexpected::Other("string"),
            Value2::ByteBuf(_) => Unexpected::Other("byte string"),
            Value2::Unit => Unexpected::Other("unit value"),
            Value2::UnitStruct(_) => Unexpected::Other("unit struct"),
            Value2::UnitVariant(_) => Unexpected::Other("unit variant"),
            Value2::RangeFull => Unexpected::Other("unit struct (RangeFull)"),
            Value2::RangeTo(_) => Unexpected::Other("struct (RangeTo)"),
            Value2::RangeToInclusive(_) => Unexpected::Other("struct (RangeToInclusive)"),
            Value2::RangeFrom(_) => Unexpected::Other("struct (RangeFrom)"),
            Value2::Range(_) => Unexpected::Other("struct (Range)"),
            Value2::RangeInclusive(_) => Unexpected::Other("struct (RangeInclusive)"),
            Value2::Maybe(_) => Unexpected::Option,
            Value2::Array(_) => Unexpected::Other("array"),
            Value2::Tuple(_) => Unexpected::Other("tuple"),
            Value2::TupleStruct(_) => Unexpected::Other("tuple struct"),
            Value2::TupleVariant(_) => Unexpected::Other("tuple variant"),
            Value2::Map(_) => Unexpected::Other("map"),
            Value2::MapStruct(_) => Unexpected::Other("map struct"),
            Value2::MapVariant(_) => Unexpected::Other("map variant"),
            Value2::Newtype(_) => Unexpected::NewtypeStruct,
        }
    }
}

struct ValueWrapper<'a>(&'a Value2);

impl<'a, 'de> Deserializer<'de> for ValueWrapper<'a> {
    type Error = ErrorImpl;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        match self.0 {
            Value2::Bool(b) => visitor.visit_bool(*b),
            Value2::Char(ch) => visitor.visit_char(*ch),
            Value2::Number(num) => num.deserializer().deserialize_any(visitor),
            Value2::String(s) => visitor.visit_str(s),
            Value2::ByteBuf(bytes) => visitor.visit_bytes(bytes),
            Value2::Unit => visitor.visit_unit(),
            Value2::UnitStruct(_) => visitor.visit_unit(),
            Value2::UnitVariant(variant) => visitor.visit_enum(VariantAccessor(variant)),
            Value2::RangeFull => visitor.visit_unit(),
            Value2::RangeTo(end) => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value2::RangeToInclusive(end) => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value2::RangeFrom(start) => visitor.visit_map(RangeAccessor {
                start: Some(**start),
                end: None,
            }),
            Value2::Range(bounds) => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value2::RangeInclusive(bounds) => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value2::Maybe(maybe) => match maybe {
                Some(value) => visitor.visit_some(value.deserializer()),
                None => visitor.visit_none(),
            },
            Value2::Array(values) => visitor.visit_seq(SeqAccessor(values.iter())),
            Value2::Tuple(values) => visitor.visit_seq(SeqAccessor(values.iter())),
            Value2::TupleStruct(r#struct) => visitor.visit_seq(SeqAccessor(r#struct.body.iter())),
            Value2::TupleVariant(variant) => visitor.visit_enum(VariantAccessor(variant)),
            Value2::Map(values_map) => visitor.visit_map(MapAccessor(values_map.iter(), None)),
            Value2::MapStruct(r#struct) => visitor.visit_map(StructAccessor(r#struct.body.iter(), None)),
            Value2::MapVariant(variant) => visitor.visit_enum(VariantAccessor(variant)),
            Value2::Newtype(r#struct) => visitor.visit_newtype_struct(r#struct.body.deserializer()),
        }
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        let _ = visitor;
        visitor.visit_none()
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> ResultKind<V::Value> {
        match self.0 {
            Value2::Array(values) | Value2::Tuple(values) => {
                if values.len() == len {
                    visitor.visit_seq(SeqAccessor(values.iter()))
                } else {
                    Err(Error::invalid_length(len, &visitor))
                }
            }
            _ => Err(Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        match self.0 {
            Value2::Map(values_map) => visitor.visit_map(MapAccessor(values_map.iter(), None)),
            Value2::MapStruct(r#struct) => visitor.visit_map(StructAccessor(r#struct.body.iter(), None)),
            _ => Err(Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        match self.0 {
            Value2::Unit => visitor.visit_unit(),
            Value2::RangeFull if name == "RangeFull" => visitor.visit_unit(),
            Value2::UnitStruct(struct_name) => {
                verify_name(struct_name.as_deref().map(AsRef::as_ref), name, &visitor)?;
                visitor.visit_unit()
            }
            _ => Err(Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        match self.0 {
            Value2::Newtype(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                r#struct.body.deserializer().deserialize_any(visitor)
            }
            Value2::TupleStruct(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                if r#struct.body.len() == 1 {
                    visitor.visit_seq(SeqAccessor(r#struct.body.iter()))
                } else {
                    Err(Error::invalid_length(1, &visitor))
                }
            }
            _ => Err(Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> ResultKind<V::Value> {
        match self.0 {
            Value2::TupleStruct(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                if r#struct.body.len() == len {
                    visitor.visit_seq(SeqAccessor(r#struct.body.iter()))
                } else {
                    Err(Error::invalid_length(len, &visitor))
                }
            }
            _ => Err(Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        let _ = fields;
        match self.0 {
            Value2::RangeTo(end) if name == "RangeTo" => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value2::RangeToInclusive(end) if name == "RangeToInclusive" => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value2::RangeFrom(start) if name == "RangeFrom" => visitor.visit_map(RangeAccessor {
                start: Some(**start),
                end: None,
            }),
            Value2::Range(bounds) if name == "Range" => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value2::RangeInclusive(bounds) if name == "RangeInclusive" => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value2::MapStruct(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                visitor.visit_map(StructAccessor(r#struct.body.iter(), None))
            }
            _ => Err(Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        let _ = variants;
        match self.0 {
            Value2::UnitVariant(variant) => {
                verify_name(variant.name.as_deref(), name, &visitor)?;
                visitor.visit_enum(VariantAccessor(variant))
            }
            Value2::TupleVariant(variant) => {
                verify_name(variant.name.as_deref(), name, &visitor)?;
                visitor.visit_enum(VariantAccessor(variant))
            }
            Value2::MapVariant(variant) => {
                verify_name(variant.name.as_deref(), name, &visitor)?;
                visitor.visit_enum(VariantAccessor(variant))
            }
            _ => Err(Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit option seq identifier
    }
}

fn verify_name<'de, V: Visitor<'de>>(name_found: Option<&Ident>, name_expected: &str, visitor: &V) -> ResultKind {
    if let Some(name_found) = name_found {
        if name_found.as_str() == name_expected {
            return Err(Error::invalid_type(Unexpected::Other(name_found), visitor));
        }
    }
    Ok(())
}

//==================================================================================================

struct VariantAccessor<'a, T>(&'a Variant<T>);

trait VariantPayload {
    fn unit(&self) -> ResultKind;
    fn tuple(&self) -> ResultKind<&Values2>;
    fn map(&self) -> ResultKind<&FieldsMap2>;
}

impl VariantPayload for () {
    fn unit(&self) -> ResultKind {
        Ok(())
    }
    fn tuple(&self) -> ResultKind<&Values2> {
        Err(Error::custom("expected unit, found tuple"))
    }
    fn map(&self) -> ResultKind<&FieldsMap2> {
        Err(Error::custom("expected unit, found struct"))
    }
}

impl VariantPayload for Values2 {
    fn unit(&self) -> ResultKind {
        Err(Error::custom("expected tuple, found unit"))
    }
    fn tuple(&self) -> ResultKind<&Values2> {
        Ok(self)
    }
    fn map(&self) -> ResultKind<&FieldsMap2> {
        Err(Error::custom("expected tuple, found struct"))
    }
}

impl VariantPayload for FieldsMap2 {
    fn unit(&self) -> ResultKind {
        Err(Error::custom("expected struct, found unit"))
    }
    fn tuple(&self) -> ResultKind<&Values2> {
        Err(Error::custom("expected struct, found tuple"))
    }
    fn map(&self) -> ResultKind<&FieldsMap2> {
        Ok(self)
    }
}

impl<'de, T: VariantPayload> EnumAccess<'de> for VariantAccessor<'_, T> {
    type Error = ErrorImpl;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> ResultKind<(V::Value, Self::Variant)> {
        Ok((
            seed.deserialize(StrDeserializer::<ErrorImpl>::new(&self.0.variant))?,
            self,
        ))
    }
}

impl<'de, T: VariantPayload> VariantAccess<'de> for VariantAccessor<'_, T> {
    type Error = ErrorImpl;

    fn unit_variant(self) -> ResultKind<()> {
        self.0.body.unit()?;
        Ok(())
    }

    fn newtype_variant_seed<U: DeserializeSeed<'de>>(self, seed: U) -> ResultKind<U::Value> {
        let tuple = self.0.body.tuple()?;
        if tuple.len() != 1 {
            return Err(Error::custom("expected newtype variant"));
        }
        seed.deserialize(tuple[0].deserializer())
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> ResultKind<V::Value> {
        let tuple = self.0.body.tuple()?;
        if tuple.len() != len {
            return Err(Error::invalid_length(len, &visitor));
        }
        visitor.visit_seq(SeqAccessor(tuple.iter()))
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> ResultKind<V::Value> {
        let _ = fields;
        let map = self.0.body.map()?;
        visitor.visit_map(StructAccessor(map.iter(), None))
    }
}

struct SeqAccessor<'a>(core::slice::Iter<'a, Value2>);

impl<'de> SeqAccess<'de> for SeqAccessor<'_> {
    type Error = ErrorImpl;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> ResultKind<Option<T::Value>> {
        let Some(val) = self.0.next() else {
            return Ok(None);
        };
        Ok(Some(seed.deserialize(val.deserializer())?))
    }
}

struct MapAccessor<'a>(btree_map::Iter<'a, Value2, Value2>, Option<&'a Value2>);

impl<'de> MapAccess<'de> for MapAccessor<'_> {
    type Error = ErrorImpl;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        let Some((key, value)) = self.0.next() else {
            return Ok(None);
        };
        self.1 = Some(value);
        Ok(Some(seed.deserialize(key.deserializer())?))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        Ok(seed.deserialize(self.1.take().expect("access value after access key").deserializer())?)
    }
}

struct StructAccessor<'a>(btree_map::Iter<'a, IdentBuf, Value2>, Option<&'a Value2>);

impl<'de> MapAccess<'de> for StructAccessor<'_> {
    type Error = ErrorImpl;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        let Some((key, value)) = self.0.next() else {
            return Ok(None);
        };
        self.1 = Some(value);
        Ok(Some(seed.deserialize(StrDeserializer::<ErrorImpl>::new(key))?))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        Ok(seed.deserialize(self.1.take().expect("access value after access key").deserializer())?)
    }
}

struct RangeAccessor {
    start: Option<Scalar>,
    end: Option<Scalar>,
}

impl<'de> MapAccess<'de> for RangeAccessor {
    type Error = ErrorImpl;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        if self.start.is_some() {
            Ok(Some(seed.deserialize(StrDeserializer::<ErrorImpl>::new("start"))?))
        } else if self.end.is_some() {
            Ok(Some(seed.deserialize(StrDeserializer::<ErrorImpl>::new("end"))?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        if let Some(scalar) = self.start.take() {
            seed.deserialize(scalar.deserializer())
        } else if let Some(scalar) = self.end.take() {
            seed.deserialize(scalar.deserializer())
        } else {
            panic!("contract violation")
        }
    }
}
