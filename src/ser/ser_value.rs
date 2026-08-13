use super::{PrivateMethod, SerializerImpl};
use crate::value::*;
use core::fmt;

impl super::Serialize for Value {
    #[expect(private_interfaces)]
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut super::Serializer<Impl>, _: PrivateMethod) -> fmt::Result {
        let ser_values = |ser: &mut super::Serializer<Impl>, values: &[Value]| -> fmt::Result {
            for value in values {
                ser.serialize_inner(value)?;
                ser.push_comma()?;
            }
            Ok(())
        };

        let ser_values_map = |ser: &mut super::Serializer<Impl>, values_map: &ValuesMap| -> fmt::Result {
            for (key, value) in values_map.iter() {
                ser.hint_map_key();
                ser.serialize_inner(key)?;
                ser.push_fat_arrow()?;
                ser.serialize_inner(value)?;
                ser.push_comma()?;
            }
            Ok(())
        };

        let ser_fields_map = |ser: &mut super::Serializer<Impl>, fields_map: &FieldsMap| -> fmt::Result {
            for (field, value) in fields_map.iter() {
                ser.push_identifier(field)?;
                ser.push_colon()?;
                ser.serialize_inner(value)?;
                ser.push_comma()?;
            }
            Ok(())
        };

        match self {
            Value::Bool(b) => ser.push_bool(*b),
            Value::Char(ch) => ser.push_char(*ch),
            Value::Number(num) => ser.push_number(num),
            Value::String(s) => ser.push_str(s),
            Value::ByteBuf(bytes) => ser.push_bytes(bytes),
            Value::Unit => ser.push_unit(),
            Value::UnitStruct(name) => ser.push_unit_struct(name.as_deref().map(AsRef::as_ref)),
            Value::UnitVariant(variant) => {
                let Variant { name, variant, .. } = variant.as_ref();
                ser.push_unit_variant(name.as_deref(), variant)
            }

            Value::RangeFull => ser.push_range_full(),
            Value::RangeTo(end) => ser.push_range_to(end),
            Value::RangeToInclusive(end) => ser.push_range_to_inclusive(end),
            Value::RangeFrom(start) => ser.push_range_from(start),
            Value::Range(start_end) => ser.push_range(&start_end.0, &start_end.1),
            Value::RangeInclusive(start_end) => ser.push_range_inclusive(&start_end.0, &start_end.1),

            Value::Maybe(maybe) => {
                ser.push_maybe_begin()?;
                if let Some(value) = maybe {
                    ser.serialize_inner(value.as_ref())?;
                }
                ser.push_maybe_end()
            }
            Value::Array(values) => {
                ser.push_array_begin()?;
                ser_values(ser, values)?;
                ser.push_array_end()
            }

            Value::Tuple(values) => {
                ser.push_tuple_begin()?;
                ser_values(ser, values)?;
                ser.push_tuple_like_end()
            }
            Value::TupleStruct(r#struct) => {
                let Struct { name, body } = r#struct.as_ref();
                ser.push_tuple_struct_begin(name.as_deref())?;
                ser_values(ser, body)?;
                ser.push_tuple_like_end()
            }
            Value::TupleVariant(variant) => {
                let Variant { name, variant, body } = variant.as_ref();
                ser.push_tuple_variant_begin(name.as_deref(), variant)?;
                ser_values(ser, body)?;
                ser.push_tuple_like_end()
            }

            Value::Map(values_map) => {
                ser.push_map_begin()?;
                ser_values_map(ser, values_map)?;
                ser.push_map_like_end()
            }
            Value::MapStruct(r#struct) => {
                let Struct { name, body } = r#struct.as_ref();
                ser.push_map_struct_begin(name.as_deref(), true)?;
                ser_fields_map(ser, body)?;
                ser.push_map_like_end()
            }
            Value::MapVariant(variant) => {
                let Variant { name, variant, body } = variant.as_ref();
                ser.push_map_variant_begin(name.as_deref(), variant)?;
                ser_fields_map(ser, body)?;
                ser.push_map_like_end()
            }

            Value::Newtype(r#struct) => {
                let Struct { name, body } = r#struct.as_ref();
                ser.push_newtype_begin(name.as_deref())?;
                ser.serialize_inner(body)?;
                ser.push_newtype_end()
            }
        }
    }
}
