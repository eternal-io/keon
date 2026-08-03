use super::{PrivateMethod, SerializerImpl};
use crate::value::*;
use core::fmt;

impl super::Serialize for Value2 {
    #[expect(private_interfaces)]
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut super::Serializer<Impl>, _: PrivateMethod) -> fmt::Result {
        let ser_values = |ser: &mut super::Serializer<Impl>, values: &[Value2]| -> fmt::Result {
            for value in values {
                ser.serialize(value)?;
                ser.push_comma()?;
            }
            Ok(())
        };

        let ser_values_map = |ser: &mut super::Serializer<Impl>, values_map: &ValuesMap2| -> fmt::Result {
            for (key, value) in values_map.iter() {
                ser.serialize(key)?;
                ser.push_fat_arrow()?;
                ser.serialize(value)?;
                ser.push_comma()?;
            }
            Ok(())
        };

        let ser_fields_map = |ser: &mut super::Serializer<Impl>, fields_map: &FieldsMap2| -> fmt::Result {
            for (field, value) in fields_map.iter() {
                ser.push_identifier(field)?;
                ser.push_colon()?;
                ser.serialize(value)?;
                ser.push_comma()?;
            }
            Ok(())
        };

        match self {
            Value2::Bool(b) => ser.push_bool(*b),
            Value2::Char(ch) => ser.push_char(*ch),
            Value2::Number(num) => match *num {
                Number2::Int8(v) => ser.push_i64(v.into(), NumberSuffix::Int8),
                Number2::Int16(v) => ser.push_i64(v.into(), NumberSuffix::Int16),
                Number2::Int32(v) => ser.push_i64(v.into(), NumberSuffix::Int32),
                Number2::Int64(v) => ser.push_i64(v.into(), NumberSuffix::Int64),
                Number2::Int128 { lo, hi } => ser.push_i128((hi as i128) << 64 | lo as i128),
                Number2::UInt8(v) => ser.push_u64(v.into(), NumberSuffix::UInt8),
                Number2::UInt16(v) => ser.push_u64(v.into(), NumberSuffix::UInt16),
                Number2::UInt32(v) => ser.push_u64(v.into(), NumberSuffix::UInt32),
                Number2::UInt64(v) => ser.push_u64(v.into(), NumberSuffix::UInt64),
                Number2::UInt128 { lo, hi } => ser.push_u128((hi as u128) << 64 | lo as u128),
                Number2::Float32(Float32(v)) => ser.push_f32(v),
                Number2::Float64(Float64(v)) => ser.push_f64(v),
                Number2::IntNoSuffix(v) => ser.push_int(v),
                Number2::UIntNoSuffix(v) => ser.push_uint(v),
                Number2::FloatNoSuffix(Float64(v)) => ser.push_float(v),
            },
            Value2::String(s) => ser.push_str(s),
            Value2::ByteBuf(bytes) => ser.push_bytes(bytes),
            Value2::Unit => ser.push_unit(),
            Value2::UnitStruct(name) => ser.push_unit_struct(name.as_deref().map(AsRef::as_ref)),
            Value2::UnitVariant(variant) => {
                let Variant { name, variant, .. } = variant.as_ref();
                ser.push_unit_variant(name.as_deref(), variant)
            }

            Value2::RangeFull => ser.push_range_full(),
            Value2::RangeTo(end) => ser.push_range_to(end),
            Value2::RangeToInclusive(end) => ser.push_range_to_inclusive(end),
            Value2::RangeFrom(start) => ser.push_range_from(start),
            Value2::Range(start_end) => ser.push_range(&start_end.0, &start_end.1),
            Value2::RangeInclusive(start_end) => ser.push_range_inclusive(&start_end.0, &start_end.1),

            Value2::Maybe(maybe) => {
                ser.push_maybe_begin()?;
                if let Some(value) = maybe {
                    ser.serialize(value.as_ref())?;
                }
                ser.push_maybe_end()
            }
            Value2::Array(values) => {
                ser.push_array_begin()?;
                ser_values(ser, values)?;
                ser.push_array_end()
            }

            Value2::Tuple(values) => {
                ser.push_tuple_begin()?;
                ser_values(ser, values)?;
                ser.push_tuple_like_end()
            }
            Value2::TupleStruct(r#struct) => {
                let Struct { name, body } = r#struct.as_ref();
                ser.push_tuple_struct_begin(name.as_deref())?;
                ser_values(ser, body)?;
                ser.push_tuple_like_end()
            }
            Value2::TupleVariant(variant) => {
                let Variant { name, variant, body } = variant.as_ref();
                ser.push_tuple_variant_begin(name.as_deref(), variant)?;
                ser_values(ser, body)?;
                ser.push_tuple_like_end()
            }

            Value2::Map(values_map) => {
                ser.push_map_begin()?;
                ser_values_map(ser, values_map)?;
                ser.push_map_like_end()
            }
            Value2::MapStruct(r#struct) => {
                let Struct { name, body } = r#struct.as_ref();
                ser.push_map_struct_begin(name.as_deref())?;
                ser_fields_map(ser, body)?;
                ser.push_map_like_end()
            }
            Value2::MapVariant(variant) => {
                let Variant { name, variant, body } = variant.as_ref();
                ser.push_map_variant_begin(name.as_deref(), variant)?;
                ser_fields_map(ser, body)?;
                ser.push_map_like_end()
            }

            Value2::Newtype(r#struct) => {
                let Struct { name, body } = r#struct.as_ref();
                ser.push_newtype_begin(name.as_deref())?;
                ser.serialize(body)?;
                ser.push_newtype_end()
            }
        }
    }
}
