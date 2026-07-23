use super::{Literal, NominalKind, PrivateMethod, SerializerImpl, Token};
use crate::value::{Struct2, Value2, ValuesMap2};
use core::fmt;

impl super::Serialize for Value2 {
    #[expect(private_interfaces)]
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut super::Serializer<Impl>, _: PrivateMethod) -> fmt::Result {
        let ser_values = |ser: &mut super::Serializer<Impl>, values: &[Value2]| -> fmt::Result {
            for value in values {
                ser.serialize(value)?;
                ser.push(Token::Comma)?;
            }
            Ok(())
        };

        let ser_values_map = |ser: &mut super::Serializer<Impl>, values_map: &ValuesMap2| -> fmt::Result {
            for (key, value) in values_map.iter() {
                ser.serialize(key)?;
                ser.push(Token::FatArrow)?;
                ser.serialize(value)?;
                ser.push(Token::Comma)?;
            }
            Ok(())
        };

        let ser_fields_map = |ser: &mut super::Serializer<Impl>, fields_map: &Struct2| -> fmt::Result {
            for (field, value) in fields_map.iter() {
                ser.push(Token::Ident(field))?;
                ser.push(Token::Colon)?;
                ser.serialize(value)?;
                ser.push(Token::Comma)?;
            }
            Ok(())
        };

        match self {
            Value2::Bool(b) => ser.push(Token::Literal(Literal::Bool(*b))),
            Value2::Char(ch) => ser.push(Token::Literal(Literal::Char(*ch))),
            Value2::Number(num) => ser.push(Token::Literal(Literal::Number(*num))),
            Value2::NumberNoSuffix(num) => ser.push(Token::Literal(Literal::NumberNoSuffix(*num))),
            Value2::String(s) => ser.push(Token::Literal(Literal::Str(s.as_ref()))),
            Value2::ByteBuf(bytes) => ser.push(Token::Literal(Literal::Bytes(bytes.as_ref()))),
            Value2::Unit => ser.push(Token::Unit),
            Value2::UnitStruct(path) => ser.push(Token::UnitStruct {
                kind: NominalKind::Unspecified,
                path: path.as_ref().into(),
            }),

            Value2::Maybe(maybe) => {
                ser.push(Token::Maybe)?;
                if let Some(value) = maybe {
                    ser.serialize(value.as_ref())?;
                }
                ser.push(Token::MaybeEnd)
            }

            Value2::Array(values) => {
                ser.push(Token::Array)?;
                ser_values(ser, values)?;
                ser.push(Token::ArrayEnd)
            }

            Value2::Tuple(values) => {
                ser.push(Token::Tuple)?;
                ser_values(ser, values)?;
                ser.push(Token::TupleLikeEnd)
            }
            Value2::TupleStruct(path_values) => {
                let (path, values) = path_values.as_ref();
                ser.push(Token::TupleStruct {
                    kind: NominalKind::Unspecified,
                    path: path.into(),
                })?;
                ser_values(ser, values)?;
                ser.push(Token::TupleLikeEnd)
            }

            Value2::Map(values_map) => {
                ser.push(Token::Map)?;
                ser_values_map(ser, values_map)?;
                ser.push(Token::MapLikeEnd)
            }
            Value2::MapStruct(path_fields_map) => {
                let (path, fields_map) = path_fields_map.as_ref();
                ser.push(Token::MapStruct {
                    kind: NominalKind::Unspecified,
                    path: path.into(),
                })?;
                ser_fields_map(ser, fields_map)?;
                ser.push(Token::MapLikeEnd)
            }
        }
    }
}
