use super::*;
use crate::de::error::{Error, Result};
use alloc::collections::btree_map;
use serde::{
    de::{
        self, value::StrDeserializer, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, Unexpected, VariantAccess,
        Visitor,
    },
    forward_to_deserialize_any, Deserialize, Deserializer,
};

impl Value {
    pub fn deserialize_to<'de, T: Deserialize<'de>>(&self) -> Result<T> {
        Ok(T::deserialize(ValueWrapper(self))?)
    }

    fn deserializer(&self) -> ValueWrapper<'_> {
        ValueWrapper(self)
    }

    fn unexpected(&self) -> Unexpected<'static> {
        match self {
            Value::Bool(_) => Unexpected::Other("boolean"),
            Value::Char(_) => Unexpected::Other("character"),
            Value::Number(_) => Unexpected::Other("number"),
            Value::String(_) => Unexpected::Other("string"),
            Value::ByteBuf(_) => Unexpected::Other("byte string"),
            Value::Unit => Unexpected::Other("unit value"),
            Value::UnitStruct(_) => Unexpected::Other("unit struct"),
            Value::UnitVariant(_) => Unexpected::Other("unit variant"),
            Value::RangeFull => Unexpected::Other("unit struct (RangeFull)"),
            Value::RangeTo(_) => Unexpected::Other("struct (RangeTo)"),
            Value::RangeToInclusive(_) => Unexpected::Other("struct (RangeToInclusive)"),
            Value::RangeFrom(_) => Unexpected::Other("struct (RangeFrom)"),
            Value::Range(_) => Unexpected::Other("struct (Range)"),
            Value::RangeInclusive(_) => Unexpected::Other("struct (RangeInclusive)"),
            Value::Maybe(_) => Unexpected::Option,
            Value::Array(_) => Unexpected::Other("array"),
            Value::Tuple(_) => Unexpected::Other("tuple"),
            Value::TupleStruct(_) => Unexpected::Other("tuple struct"),
            Value::TupleVariant(_) => Unexpected::Other("tuple variant"),
            Value::Map(_) => Unexpected::Other("map"),
            Value::MapStruct(_) => Unexpected::Other("map struct"),
            Value::MapVariant(_) => Unexpected::Other("map variant"),
            Value::Newtype(_) => Unexpected::NewtypeStruct,
        }
    }
}

struct ValueWrapper<'a>(&'a Value);

impl<'a, 'de> Deserializer<'de> for ValueWrapper<'a> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.0 {
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Char(ch) => visitor.visit_char(*ch),
            Value::Number(num) => num.deserializer().deserialize_any(visitor),
            Value::String(s) => visitor.visit_str(s),
            Value::ByteBuf(bytes) => visitor.visit_bytes(bytes),
            Value::Unit => visitor.visit_unit(),
            Value::UnitStruct(_) => visitor.visit_unit(),
            Value::UnitVariant(variant) => visitor.visit_enum(EnumAccessor(variant)),
            Value::RangeFull => visitor.visit_unit(),
            Value::RangeTo(end) => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value::RangeToInclusive(end) => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value::RangeFrom(start) => visitor.visit_map(RangeAccessor {
                start: Some(**start),
                end: None,
            }),
            Value::Range(bounds) => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value::RangeInclusive(bounds) => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value::Maybe(maybe) => match maybe {
                Some(value) => visitor.visit_some(value.deserializer()),
                None => visitor.visit_none(),
            },
            Value::Array(values) => visitor.visit_seq(SeqAccessor(values.iter())),
            Value::Tuple(values) => visitor.visit_seq(SeqAccessor(values.iter())),
            Value::TupleStruct(r#struct) => visitor.visit_seq(SeqAccessor(r#struct.body.iter())),
            Value::TupleVariant(variant) => visitor.visit_enum(EnumAccessor(variant)),
            Value::Map(values_map) => visitor.visit_map(MapAccessor(values_map.iter(), None)),
            Value::MapStruct(r#struct) => visitor.visit_map(StructAccessor(r#struct.body.iter(), None)),
            Value::MapVariant(variant) => visitor.visit_enum(EnumAccessor(variant)),
            Value::Newtype(r#struct) => visitor.visit_newtype_struct(r#struct.body.deserializer()),
        }
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let _ = visitor;
        visitor.visit_none()
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        match self.0 {
            Value::Array(values) | Value::Tuple(values) => {
                if values.len() == len {
                    visitor.visit_seq(SeqAccessor(values.iter()))
                } else {
                    Err(de::Error::invalid_length(len, &visitor))
                }
            }
            _ => Err(de::Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.0 {
            Value::Map(values_map) => visitor.visit_map(MapAccessor(values_map.iter(), None)),
            Value::MapStruct(r#struct) => visitor.visit_map(StructAccessor(r#struct.body.iter(), None)),
            _ => Err(de::Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value> {
        match self.0 {
            Value::Unit => visitor.visit_unit(),
            Value::RangeFull if name == "RangeFull" => visitor.visit_unit(),
            Value::UnitStruct(struct_name) => {
                verify_name(struct_name.as_deref().map(AsRef::as_ref), name, &visitor)?;
                visitor.visit_unit()
            }
            _ => Err(de::Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value> {
        match self.0 {
            Value::Newtype(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                r#struct.body.deserializer().deserialize_any(visitor)
            }
            Value::TupleStruct(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                if r#struct.body.len() == 1 {
                    visitor.visit_seq(SeqAccessor(r#struct.body.iter()))
                } else {
                    Err(de::Error::invalid_length(1, &visitor))
                }
            }
            _ => Err(de::Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value> {
        match self.0 {
            Value::TupleStruct(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                if r#struct.body.len() == len {
                    visitor.visit_seq(SeqAccessor(r#struct.body.iter()))
                } else {
                    Err(de::Error::invalid_length(len, &visitor))
                }
            }
            _ => Err(de::Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let _ = fields;
        match self.0 {
            Value::RangeTo(end) if name == "RangeTo" => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value::RangeToInclusive(end) if name == "RangeToInclusive" => visitor.visit_map(RangeAccessor {
                start: None,
                end: Some(**end),
            }),
            Value::RangeFrom(start) if name == "RangeFrom" => visitor.visit_map(RangeAccessor {
                start: Some(**start),
                end: None,
            }),
            Value::Range(bounds) if name == "Range" => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value::RangeInclusive(bounds) if name == "RangeInclusive" => visitor.visit_map(RangeAccessor {
                start: Some(bounds.0),
                end: Some(bounds.1),
            }),
            Value::MapStruct(r#struct) => {
                verify_name(r#struct.name.as_deref(), name, &visitor)?;
                visitor.visit_map(StructAccessor(r#struct.body.iter(), None))
            }
            _ => Err(de::Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let _ = variants;
        match self.0 {
            Value::UnitVariant(variant) => {
                verify_name(variant.name.as_deref(), name, &visitor)?;
                visitor.visit_enum(EnumAccessor(variant))
            }
            Value::TupleVariant(variant) => {
                verify_name(variant.name.as_deref(), name, &visitor)?;
                visitor.visit_enum(EnumAccessor(variant))
            }
            Value::MapVariant(variant) => {
                verify_name(variant.name.as_deref(), name, &visitor)?;
                visitor.visit_enum(EnumAccessor(variant))
            }
            _ => Err(de::Error::invalid_type(self.0.unexpected(), &visitor)),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit option seq identifier
    }
}

fn verify_name<'de, V: Visitor<'de>>(name_found: Option<&Ident>, name_expected: &str, visitor: &V) -> Result {
    if let Some(name_found) = name_found {
        if name_found.as_str() == name_expected {
            return Err(de::Error::invalid_type(Unexpected::Other(name_found), visitor));
        }
    }
    Ok(())
}

//==================================================================================================

struct EnumAccessor<'a, T>(&'a Variant<T>);

trait VariantPayload {
    fn unit(&self) -> Result;
    fn tuple(&self) -> Result<&Values>;
    fn map(&self) -> Result<&FieldsMap>;
}

impl VariantPayload for () {
    fn unit(&self) -> Result {
        Ok(())
    }
    fn tuple(&self) -> Result<&Values> {
        Err(de::Error::custom("expected unit, found tuple"))
    }
    fn map(&self) -> Result<&FieldsMap> {
        Err(de::Error::custom("expected unit, found struct"))
    }
}

impl VariantPayload for Values {
    fn unit(&self) -> Result {
        Err(de::Error::custom("expected tuple, found unit"))
    }
    fn tuple(&self) -> Result<&Values> {
        Ok(self)
    }
    fn map(&self) -> Result<&FieldsMap> {
        Err(de::Error::custom("expected tuple, found struct"))
    }
}

impl VariantPayload for FieldsMap {
    fn unit(&self) -> Result {
        Err(de::Error::custom("expected struct, found unit"))
    }
    fn tuple(&self) -> Result<&Values> {
        Err(de::Error::custom("expected struct, found tuple"))
    }
    fn map(&self) -> Result<&FieldsMap> {
        Ok(self)
    }
}

impl<'de, T: VariantPayload> EnumAccess<'de> for EnumAccessor<'_, T> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        Ok((seed.deserialize(StrDeserializer::<Error>::new(&self.0.variant))?, self))
    }
}

impl<'de, T: VariantPayload> VariantAccess<'de> for EnumAccessor<'_, T> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        self.0.body.unit()?;
        Ok(())
    }

    fn newtype_variant_seed<U: DeserializeSeed<'de>>(self, seed: U) -> Result<U::Value> {
        let tuple = self.0.body.tuple()?;
        if tuple.len() != 1 {
            return Err(de::Error::custom("expected newtype variant"));
        }
        seed.deserialize(tuple[0].deserializer())
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        let tuple = self.0.body.tuple()?;
        if tuple.len() != len {
            return Err(de::Error::invalid_length(len, &visitor));
        }
        visitor.visit_seq(SeqAccessor(tuple.iter()))
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        let _ = fields;
        let map = self.0.body.map()?;
        visitor.visit_map(StructAccessor(map.iter(), None))
    }
}

struct SeqAccessor<'a>(core::slice::Iter<'a, Value>);

impl<'de> SeqAccess<'de> for SeqAccessor<'_> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        let Some(val) = self.0.next() else {
            return Ok(None);
        };
        Ok(Some(seed.deserialize(val.deserializer())?))
    }
}

struct MapAccessor<'a>(btree_map::Iter<'a, Value, Value>, Option<&'a Value>);

impl<'de> MapAccess<'de> for MapAccessor<'_> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        let Some((key, value)) = self.0.next() else {
            return Ok(None);
        };
        self.1 = Some(value);
        Ok(Some(seed.deserialize(key.deserializer())?))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(self.1.take().expect("access value after access key").deserializer())
    }
}

struct StructAccessor<'a>(btree_map::Iter<'a, IdentBuf, Value>, Option<&'a Value>);

impl<'de> MapAccess<'de> for StructAccessor<'_> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        let Some((key, value)) = self.0.next() else {
            return Ok(None);
        };
        self.1 = Some(value);
        Ok(Some(seed.deserialize(StrDeserializer::<Error>::new(key))?))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(self.1.take().expect("access value after access key").deserializer())
    }
}

struct RangeAccessor {
    start: Option<Scalar>,
    end: Option<Scalar>,
}

impl<'de> MapAccess<'de> for RangeAccessor {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.start.is_some() {
            Ok(Some(seed.deserialize(StrDeserializer::<Error>::new("start"))?))
        } else if self.end.is_some() {
            Ok(Some(seed.deserialize(StrDeserializer::<Error>::new("end"))?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        if let Some(scalar) = self.start.take() {
            seed.deserialize(scalar.deserializer())
        } else if let Some(scalar) = self.end.take() {
            seed.deserialize(scalar.deserializer())
        } else {
            panic!("contract violation")
        }
    }
}
