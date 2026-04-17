use super::{private::*, SerializerImpl};
use crate::value::Value2;
use core::fmt;

impl super::Serialize for Value2 {
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut super::Serializer<Impl>) -> fmt::Result {
        let ser_values = |ser: &mut super::Serializer<Impl>, values: &[Value2]| -> fmt::Result {
            for value in values {
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
                path: path.as_ref().into(),
                kind: NominalKind::Preserve,
            }),

            Value2::Maybe(maybe) => {
                ser.push(Token::Maybe)?;
                if let Some(value) = maybe.as_ref() {
                    ser.serialize(value.as_ref())?;
                }
                ser.push(Token::MaybeEnd)
            }

            Value2::Sequence(values) => {
                ser.push(Token::Sequence)?;
                ser_values(ser, values.as_ref())?;
                ser.push(Token::SequenceEnd)
            }

            Value2::Tuple(values) => {
                ser.push(Token::Tuple)?;
                ser_values(ser, values.as_ref())?;
                ser.push(Token::TupleLikeEnd)
            }
            Value2::TupleStruct(path_values) => {
                let (path, values) = path_values.as_ref();
                ser.push(Token::TupleStruct {
                    path: path.into(),
                    kind: NominalKind::Preserve,
                })?;
                ser_values(ser, values.as_ref())?;
                ser.push(Token::TupleLikeEnd)
            }

            Value2::Map(values_map) => {
                ser.push(Token::Map)?;
                for (key, value) in values_map.iter() {
                    ser.serialize(key)?;
                    ser.push(Token::FatArrow)?;
                    ser.serialize(value)?;
                    ser.push(Token::Comma)?;
                }
                ser.push(Token::MapLikeEnd)
            }
            Value2::MapStruct(path_values_map) => {
                let (path, values_map) = path_values_map.as_ref();
                ser.push(Token::MapStruct {
                    path: path.into(),
                    kind: NominalKind::Preserve,
                })?;
                for (key, value) in values_map.iter() {
                    ser.push(Token::Ident(key.as_ref()))?;
                    ser.push(Token::Colon)?;
                    ser.serialize(value)?;
                    ser.push(Token::Comma)?;
                }
                ser.push(Token::MapLikeEnd)
            }

            Value2::Nominal { path, stru } => todo!(),
        }
    }
}
