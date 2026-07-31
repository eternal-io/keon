use super::*;
use core::fmt;
use serde::ser::{
    Error, Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer, StdError,
};

impl<T: Serialize> From<T> for Value2 {
    fn from(value: T) -> Self {
        value.serialize(MakeValue).unwrap()
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
    type Ok = Value2;
    type Error = Never;
    type SerializeSeq = MakeValues;
    type SerializeTuple = MakeValues;
    type SerializeTupleStruct = MakeValues;
    type SerializeTupleVariant = MakeValues;
    type SerializeMap = MakeValuesMap;
    type SerializeStruct = MakeFieldsMap;
    type SerializeStructVariant = MakeFieldsMap;

    #[rustfmt::skip]    fn serialize_bool  (self, v: bool ) -> Fine<Value2> { Ok(Value2::Bool(v))           }
    #[rustfmt::skip]    fn serialize_char  (self, v: char ) -> Fine<Value2> { Ok(Value2::Char(v))           }
    #[rustfmt::skip]    fn serialize_i8    (self, v: i8   ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i16   (self, v: i16  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i32   (self, v: i32  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i64   (self, v: i64  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_i128  (self, v: i128 ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u8    (self, v: u8   ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u16   (self, v: u16  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u32   (self, v: u32  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u64   (self, v: u64  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_u128  (self, v: u128 ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_f32   (self, v: f32  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_f64   (self, v: f64  ) -> Fine<Value2> { Ok(Value2::Number(v.into()))  }
    #[rustfmt::skip]    fn serialize_str   (self, v: &str ) -> Fine<Value2> { Ok(Value2::String(v.into()))  }
    #[rustfmt::skip]    fn serialize_bytes (self, v: &[u8]) -> Fine<Value2> { Ok(Value2::ByteBuf(v.into())) }

    fn serialize_unit(self) -> Fine<Value2> {
        Ok(Value2::Unit)
    }
    fn serialize_unit_struct(self, name: &'static str) -> Fine<Value2> {
        if name == "RangeFull" {
            Ok(Value2::RangeFull)
        } else {
            Ok(Value2::UnitStruct(Box::new(NominalPath2::Single {
                name: Ident::new_unchecked(name),
            })))
        }
    }
    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Fine<Value2> {
        let _ = variant_index;
        Ok(Value2::UnitStruct(Box::new(NominalPath2::Dual {
            name: Ident::new_unchecked(variant),
            parent: Ident::new_unchecked(name),
        })))
    }

    fn serialize_none(self) -> Fine<Value2> {
        Ok(Value2::Maybe(None))
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Fine<Value2> {
        Ok(Value2::Maybe(Some(Box::new(value.serialize(MakeValue)?))))
    }
    fn serialize_seq(self, len: Option<usize>) -> Fine<Self::SerializeSeq> {
        Ok(MakeValues::new(len.unwrap_or(8), None))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, name: &'static str, value: &T) -> Fine<Value2> {
        Ok(Value2::Newtype(Box::new((
            Ident::new_unchecked(name),
            value.serialize(MakeValue)?,
        ))))
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Fine<Value2> {
        let _ = variant_index;
        Ok(Value2::TupleStruct(Box::new((
            NominalPath2::Dual {
                name: Ident::new_unchecked(variant),
                parent: Ident::new_unchecked(name),
            },
            vec![value.serialize(MakeValue)?],
        ))))
    }

    fn serialize_tuple(self, len: usize) -> Fine<Self::SerializeTuple> {
        Ok(MakeValues::new(len, None))
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Fine<Self::SerializeTupleStruct> {
        Ok(MakeValues::new(
            len,
            Some(NominalPath2::Single {
                name: Ident::new_unchecked(name),
            }),
        ))
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Fine<Self::SerializeTupleVariant> {
        let _ = variant_index;
        Ok(MakeValues::new(
            len,
            Some(NominalPath2::Dual {
                name: Ident::new_unchecked(variant),
                parent: Ident::new_unchecked(name),
            }),
        ))
    }

    fn serialize_map(self, len: Option<usize>) -> Fine<Self::SerializeMap> {
        let _ = len;
        Ok(MakeValuesMap::new())
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Fine<Self::SerializeStruct> {
        let _ = len;
        Ok(MakeFieldsMap::new(NominalPath2::Single {
            name: Ident::new_unchecked(name),
        }))
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
        Ok(MakeFieldsMap::new(NominalPath2::Dual {
            name: Ident::new_unchecked(variant),
            parent: Ident::new_unchecked(name),
        }))
    }
}

struct MakeValues {
    path: Option<NominalPath2>,
    vals: Values2,
}
impl MakeValues {
    fn new(len: usize, path: Option<NominalPath2>) -> Self {
        Self {
            path,
            vals: Values2::with_capacity(len),
        }
    }
}
impl SerializeSeq for MakeValues {
    type Ok = Value2;
    type Error = Never;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value2> {
        Ok(Value2::Array(Box::new(self.vals)))
    }
}
impl SerializeTuple for MakeValues {
    type Ok = Value2;
    type Error = Never;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value2> {
        Ok(Value2::Tuple(Box::new(self.vals)))
    }
}
impl SerializeTupleStruct for MakeValues {
    type Ok = Value2;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value2> {
        Ok(Value2::TupleStruct(Box::new((
            self.path.expect("nominal path"),
            self.vals,
        ))))
    }
}
impl SerializeTupleVariant for MakeValues {
    type Ok = Value2;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        Ok(self.vals.push(value.serialize(MakeValue)?))
    }
    fn end(self) -> Fine<Value2> {
        Ok(Value2::TupleStruct(Box::new((
            self.path.expect("nominal path"),
            self.vals,
        ))))
    }
}

struct MakeValuesMap {
    values_map: ValuesMap2,
    last_key: Option<Value2>,
}
impl MakeValuesMap {
    fn new() -> Self {
        Self {
            values_map: ValuesMap2::new(),
            last_key: None,
        }
    }
}
impl SerializeMap for MakeValuesMap {
    type Ok = Value2;
    type Error = Never;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Fine {
        self.last_key = Some(key.serialize(MakeValue)?);
        Ok(())
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Fine {
        self.values_map.insert(
            self.last_key.take().expect("serialize value after serialize key"),
            value.serialize(MakeValue)?,
        );
        Ok(())
    }
    fn end(self) -> Fine<Value2> {
        Ok(Value2::Map(Box::new(self.values_map)))
    }
}

struct MakeFieldsMap {
    path: NominalPath2,
    fields_map: Struct2,
}
impl MakeFieldsMap {
    fn new(path: NominalPath2) -> Self {
        Self {
            path,
            fields_map: Struct2::new(),
        }
    }
}
impl SerializeStruct for MakeFieldsMap {
    type Ok = Value2;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Fine {
        self.fields_map
            .insert(Ident::new_unchecked(key), value.serialize(MakeValue)?);
        Ok(())
    }
    fn end(self) -> Fine<Value2> {
        let NominalPath2::Single { ref name } = self.path else {
            unreachable!()
        };
        'range_type: {
            const KEY_START: &IdentRef<'static> = &IdentRef::new_unchecked("start");
            const KEY_END: &IdentRef<'static> = &IdentRef::new_unchecked("end");

            let len = self.fields_map.len();
            if len != 1 && len != 2 {
                break 'range_type;
            }

            let has_start = self.fields_map.contains_key(KEY_START);
            let has_end = self.fields_map.contains_key(KEY_END);

            if len == 1 && has_start {
                let range_value = match name.as_ref() {
                    "RangeFrom" => Value2::RangeFrom,
                    _ => break 'range_type,
                };

                if let Ok(start) = Scalar::try_from(&self.fields_map[KEY_START]) {
                    return Ok(range_value(Box::new(start)));
                }
            } else if len == 1 && has_end {
                let range_value = match name.as_ref() {
                    "RangeTo" => Value2::RangeTo,
                    "RangeToInclusive" => Value2::RangeToInclusive,
                    _ => break 'range_type,
                };

                if let Ok(end) = Scalar::try_from(&self.fields_map[KEY_END]) {
                    return Ok(range_value(Box::new(end)));
                }
            } else if len == 2 && has_start && has_end {
                let range_value = match name.as_ref() {
                    "Range" => Value2::Range,
                    "RangeInclusive" => Value2::RangeInclusive,
                    _ => break 'range_type,
                };

                if let Ok(start) = Scalar::try_from(&self.fields_map[KEY_START]) {
                    if let Ok(end) = Scalar::try_from(&self.fields_map[KEY_END]) {
                        return Ok(range_value(Box::new((start, end))));
                    }
                }
            }
        }
        Ok(Value2::MapStruct(Box::new((self.path, self.fields_map))))
    }
}
impl SerializeStructVariant for MakeFieldsMap {
    type Ok = Value2;
    type Error = Never;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Fine {
        self.fields_map
            .insert(Ident::new_unchecked(key), value.serialize(MakeValue)?);
        Ok(())
    }
    fn end(self) -> Fine<Value2> {
        Ok(Value2::MapStruct(Box::new((self.path, self.fields_map))))
    }
}
