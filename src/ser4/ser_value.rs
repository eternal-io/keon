use super::{private::*, SerializerImpl};
use crate::value::Value2;
use core::fmt;

impl super::Serialize for Value2 {
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut super::Serializer<Impl>) -> fmt::Result {
        match self {
            Value2::Bool(b) => ser.0.push(Token::Literal(Literal::Bool(*b))),
            Value2::Char(ch) => ser.0.push(Token::Literal(Literal::Char(*ch))),
            Value2::Number(num) => ser.0.push(Token::Literal(Literal::Number(*num))),
            Value2::NumberNoSuffix(num) => ser.0.push(Token::Literal(Literal::NumberNoSuffix(*num))),
            Value2::String(s) => ser.0.push(Token::Literal(Literal::Str(s.as_ref()))),
            Value2::ByteBuf(bytes) => ser.0.push(Token::Literal(Literal::Bytes(bytes.as_ref()))),

            Value2::Maybe(value2) => todo!(),
            Value2::Sequence(value2s) => todo!(),
            Value2::Tuple(value2s) => todo!(),
            Value2::TupleStruct(_) => todo!(),
            Value2::Map(btree_map) => todo!(),
            Value2::MapStruct(_) => todo!(),
        }
    }
}
