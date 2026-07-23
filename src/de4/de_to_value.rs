use super::{error::*, source::*, Deserialize, Deserializer, PrivateMethod};
use crate::value::{Ident, Struct2, Value2, Values2, ValuesMap2};

impl<'de> Deserialize<'de> for Value2 {
    #[expect(private_interfaces)]
    fn deserialize_with<R: Source<'de>>(der: &mut Deserializer<R>, _: PrivateMethod) -> ResultKind<Self> {
        Ok(match der.src.begin()? {
            Indicator::Unit => Self::Unit,
            Indicator::Bool(b) => Self::Bool(b),
            Indicator::Char(ch) => Self::Char(ch),
            Indicator::Byte(byte) => Self::NumberNoSuffix(byte.into()),
            Indicator::String(kind) => Self::String(der.src.parse_string(kind, &mut der.buf)?.either_into()),
            Indicator::Bytes(kind) => Self::ByteBuf(der.src.parse_bytes(kind, &mut der.buf)?.either_into()),
            Indicator::Number(kind) => match kind {
                NumberKind::Normal => der.src.parse_number(kind)?.either_into(),
                NumberKind::Infinity => Self::NumberNoSuffix(f64::INFINITY.into()),
                NumberKind::NegInfinity => Self::NumberNoSuffix(f64::NEG_INFINITY.into()),
                NumberKind::NotANumber => Self::NumberNoSuffix(f64::NAN.into()),
            },
            Indicator::Initiator(init) => match init {
                Initiator::Maybe => Self::Maybe(if !der.src.adjacent_to_delim()? {
                    Some(Box::new(deserialize_value(der)?))
                } else {
                    None
                }),
                Initiator::Array => {
                    let val = Self::Array(Box::new(deserialize_values(der)?));
                    der.src.end_array()?;
                    val
                }
                Initiator::Tuple => {
                    let val = Self::Tuple(Box::new(deserialize_values(der)?));
                    der.src.end_tuple()?;
                    val
                }
                Initiator::MapLike => {
                    let val = Self::Map(Box::new(deserialize_values_map(der)?));
                    der.src.end_map_like()?;
                    val
                }
            },
            Indicator::NominalPath(path) => match der.src.initiator()? {
                Some(init) => match init {
                    Initiator::Tuple => {
                        let val = Self::TupleStruct(Box::new((path.into(), deserialize_values(der)?)));
                        der.src.end_tuple()?;
                        val
                    }
                    Initiator::MapLike => {
                        let val = Self::MapStruct(Box::new((path.into(), deserialize_fields_map(der)?)));
                        der.src.end_map_like()?;
                        val
                    }
                    _ => return Err(ErrorKind::InvalidNominalStructureBody),
                },
                None => Self::UnitStruct(Box::new(path.into())),
            },
        })
    }
}

fn deserialize_value<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<Value2> {
    der.ttl_enter()?;
    let value = Value2::deserialize_with(der, PrivateMethod)?;
    der.ttl_leave();
    Ok(value)
}

fn deserialize_values<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<Values2> {
    der.ttl_enter()?;
    let mut values = Values2::new();
    loop {
        if der.src.adjacent_to_delim()? {
            break;
        }
        let value = Value2::deserialize_with(der, PrivateMethod)?;
        der.src.delim(Delimiter::Comma)?;
        values.push(value);
    }
    der.ttl_leave();
    Ok(values)
}

fn deserialize_values_map<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<ValuesMap2> {
    der.ttl_enter()?;
    let mut values_map = ValuesMap2::new();
    loop {
        if der.src.adjacent_to_delim()? {
            break;
        }
        let key = Value2::deserialize_with(der, PrivateMethod)?;
        der.src.delim_expected(Delimiter::Colon, ErrorKind::ExpectedColon)?;
        let value = Value2::deserialize_with(der, PrivateMethod)?;
        der.src.delim(Delimiter::Comma)?;
        values_map.insert(key, value);
    }
    der.ttl_leave();
    Ok(values_map)
}

fn deserialize_fields_map<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<Struct2> {
    der.ttl_enter()?;
    let mut fields_map = Struct2::new();
    loop {
        if der.src.adjacent_to_delim()? {
            break;
        }
        let field = Ident::from(der.src.parse_identifier(&mut der.buf)?);
        der.src
            .delim_expected(Delimiter::FatArrow, ErrorKind::ExpectedFatArrow)?;
        let value = Value2::deserialize_with(der, PrivateMethod)?;
        der.src.delim(Delimiter::Comma)?;
        fields_map.insert(field, value);
    }
    der.ttl_leave();
    Ok(fields_map)
}
