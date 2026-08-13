use super::*;
use alloc::vec;
use core::fmt;
use serde::ser::{
    Error, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer, StdError,
};

impl<T: Serialize> From<T> for Value {
    fn from(value: T) -> Self {
        value.serialize(MakeValue).expect("never fails")
    }
}

type Fine<T = ()> = Result<T, Never>;

enum Never {}

impl StdError for Never {}

impl fmt::Debug for Never {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            _ => Ok(()),
        }
    }
}

impl fmt::Display for Never {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            _ => Ok(()),
        }
    }
}

impl Error for Never {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        unreachable!()
    }
}

struct MakeValue;

impl Serializer for MakeValue {
    type Ok = Value;
    type Error = Never;
    type SerializeSeq = MakeTuple;
    type SerializeTuple = MakeTuple;
    type SerializeTupleStruct = MakeTupleStruct;
    type SerializeTupleVariant = MakeTupleVariant;
    type SerializeMap = MakeMap;
    type SerializeStruct = MakeMapStruct;
    type SerializeStructVariant = MakeMapVariant;

    #[rustfmt::skip]    fn serialize_bool  (self, v: bool ) -> Fine<Value> { Ok(Value::Bool(v))           }
    #[rustfmt::skip]    fn serialize_char  (self, v: char ) -> Fine<Value> { Ok(Value::Char(v))           }
    #[rustfmt::skip]    fn serialize_i8    (self, v: i8   ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i16   (self, v: i16  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i32   (self, v: i32  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i64   (self, v: i64  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i128  (self, v: i128 ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u8    (self, v: u8   ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u16   (self, v: u16  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u32   (self, v: u32  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u64   (self, v: u64  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u128  (self, v: u128 ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_f32   (self, v: f32  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_f64   (self, v: f64  ) -> Fine<Value> { Ok(Value::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_str   (self, v: &str ) -> Fine<Value> { Ok(Value::String(v.into()))  }
    #[rustfmt::skip]    fn serialize_bytes (self, v: &[u8]) -> Fine<Value> { Ok(Value::ByteBuf(v.into())) }

    fn serialize_unit(self) -> Fine<Value> {
        Ok(Value::Unit)
    }
    fn serialize_unit_struct(self, name: &'static str) -> Fine<Value> {
        if name == "RangeFull" {
            Ok(Value::RangeFull)
        } else {
            Ok(Value::UnitStruct(Some(Box::new(Ident::new_unchecked(name).to_owned()))))
        }
    }
    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Fine<Value> {
        let _ = variant_index;
        Ok(Value::UnitVariant(Box::new(Variant {
            name: Some(Ident::new_unchecked(name).to_owned()),
            variant: Ident::new_unchecked(variant).to_owned(),
            body: (),
        })))
    }

    fn serialize_none(self) -> Fine<Value> {
        Ok(Value::Maybe(None))
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Fine<Value> {
        Ok(Value::Maybe(Some(Box::new(value.serialize(MakeValue)?))))
    }
    fn serialize_seq(self, len: Option<usize>) -> Fine<Self::SerializeSeq> {
        Ok(MakeTuple::new(len.unwrap_or(8)))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, name: &'static str, value: &T) -> Fine<Value> {
        Ok(Value::Newtype(Box::new(Struct {
            name: Some(Ident::new_unchecked(name).to_owned()),
            body: value.serialize(MakeValue)?,
        })))
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Fine<Value> {
        let _ = variant_index;
        Ok(Value::TupleVariant(Box::new(Variant {
            name: Some(Ident::new_unchecked(name).to_owned()),
            variant: Ident::new_unchecked(variant).to_owned(),
            body: vec![value.serialize(MakeValue)?],
        })))
    }

    fn serialize_tuple(self, len: usize) -> Fine<Self::SerializeTuple> {
        Ok(MakeTuple::new(len))
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Fine<Self::SerializeTupleStruct> {
        Ok(MakeTupleStruct::new(len, Ident::new_unchecked(name).to_owned()))
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Fine<Self::SerializeTupleVariant> {
        let _ = variant_index;
        Ok(MakeTupleVariant::new(
            len,
            Ident::new_unchecked(name).to_owned(),
            Ident::new_unchecked(variant).to_owned(),
        ))
    }

    fn serialize_map(self, len: Option<usize>) -> Fine<Self::SerializeMap> {
        let _ = len;
        Ok(MakeMap::new())
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Fine<Self::SerializeStruct> {
        let _ = len;
        Ok(MakeMapStruct::new(Ident::new_unchecked(name).to_owned()))
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Fine<Self::SerializeStructVariant> {
        let _ = len;
        let _ = variant_index;
        Ok(MakeMapVariant::new(
            Ident::new_unchecked(name).to_owned(),
            Ident::new_unchecked(variant).to_owned(),
        ))
    }
}

struct MakeTuple {
    vals: Values,
}
impl MakeTuple {
    fn new(len: usize) -> Self {
        Self {
            vals: Values::with_capacity(len),
        }
    }
}
impl SerializeSeq for MakeTuple {
    type Ok = Value;
    type Error = Never;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value> {
        Ok(Value::Array(Box::new(self.vals)))
    }
}
impl SerializeTuple for MakeTuple {
    type Ok = Value;
    type Error = Never;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value> {
        Ok(Value::Tuple(Box::new(self.vals)))
    }
}

struct MakeTupleStruct {
    name: IdentBuf,
    vals: Values,
}
impl MakeTupleStruct {
    fn new(len: usize, name: IdentBuf) -> Self {
        Self {
            name,
            vals: Values::with_capacity(len),
        }
    }
}
impl SerializeTupleStruct for MakeTupleStruct {
    type Ok = Value;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value> {
        Ok(Value::TupleStruct(Box::new(Struct {
            name: Some(self.name),
            body: self.vals,
        })))
    }
}

struct MakeTupleVariant {
    name: IdentBuf,
    variant: IdentBuf,
    vals: Values,
}
impl MakeTupleVariant {
    fn new(len: usize, name: IdentBuf, variant: IdentBuf) -> Self {
        Self {
            name,
            variant,
            vals: Values::with_capacity(len),
        }
    }
}
impl SerializeTupleVariant for MakeTupleVariant {
    type Ok = Value;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value> {
        Ok(Value::TupleVariant(Box::new(Variant {
            name: Some(self.name),
            variant: self.variant,
            body: self.vals,
        })))
    }
}

struct MakeMap {
    vals: ValuesMap,
    last_key: Option<Value>,
}
impl MakeMap {
    fn new() -> Self {
        Self {
            vals: ValuesMap::new(),
            last_key: None,
        }
    }
}
impl SerializeMap for MakeMap {
    type Ok = Value;
    type Error = Never;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Fine {
        self.last_key = Some(key.serialize(MakeValue)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        self.vals.insert(
            self.last_key.take().expect("serialize value after serialize key"),
            value.serialize(MakeValue)?,
        );
        Ok(())
    }
    fn end(self) -> Fine<Value> {
        Ok(Value::Map(Box::new(self.vals)))
    }
}

struct MakeMapStruct {
    name: IdentBuf,
    vals: FieldsMap,
}
impl MakeMapStruct {
    fn new(name: IdentBuf) -> Self {
        Self {
            name,
            vals: FieldsMap::new(),
        }
    }
}
impl SerializeStruct for MakeMapStruct {
    type Ok = Value;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Fine {
        self.vals
            .insert(Ident::new_unchecked(key).to_owned(), value.serialize(MakeValue)?);
        Ok(())
    }
    fn end(self) -> Fine<Value> {
        'range_type: {
            const KEY_START: &Ident = Ident::new_unchecked("start");
            const KEY_END: &Ident = Ident::new_unchecked("end");

            let len = self.vals.len();
            if len != 1 && len != 2 {
                break 'range_type;
            }

            let has_start = self.vals.contains_key(KEY_START);
            let has_end = self.vals.contains_key(KEY_END);

            let name = self.name.as_str();
            if len == 1 && has_start {
                let range_value = match name {
                    "RangeFrom" => Value::RangeFrom,
                    _ => break 'range_type,
                };

                if let Ok(start) = Scalar::try_from(&self.vals[KEY_START]) {
                    return Ok(range_value(Box::new(start)));
                }
            } else if len == 1 && has_end {
                let range_value = match name {
                    "RangeTo" => Value::RangeTo,
                    "RangeToInclusive" => Value::RangeToInclusive,
                    _ => break 'range_type,
                };

                if let Ok(end) = Scalar::try_from(&self.vals[KEY_END]) {
                    return Ok(range_value(Box::new(end)));
                }
            } else if len == 2 && has_start && has_end {
                let range_value = match name {
                    "Range" => Value::Range,
                    "RangeInclusive" => Value::RangeInclusive,
                    _ => break 'range_type,
                };

                if let Ok(start) = Scalar::try_from(&self.vals[KEY_START]) {
                    if let Ok(end) = Scalar::try_from(&self.vals[KEY_END]) {
                        return Ok(range_value(Box::new((start, end))));
                    }
                }
            }
        }
        Ok(Value::MapStruct(Box::new(Struct {
            name: Some(self.name),
            body: self.vals,
        })))
    }
}

struct MakeMapVariant {
    name: IdentBuf,
    variant: IdentBuf,
    vals: FieldsMap,
}
impl MakeMapVariant {
    fn new(name: IdentBuf, variant: IdentBuf) -> Self {
        Self {
            name,
            variant,
            vals: FieldsMap::new(),
        }
    }
}
impl SerializeStructVariant for MakeMapVariant {
    type Ok = Value;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Fine {
        self.vals
            .insert(Ident::new_unchecked(key).to_owned(), value.serialize(MakeValue)?);
        Ok(())
    }
    fn end(self) -> Fine<Value> {
        Ok(Value::MapVariant(Box::new(Variant {
            name: Some(self.name),
            variant: self.variant,
            body: self.vals,
        })))
    }
}
