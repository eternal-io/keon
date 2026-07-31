use super::{error::*, source::*, Deserialize, Deserializer, PrivateMethod};
use crate::value::{Ident, Scalar, Struct2, Value2, Values2, ValuesMap2};
use either::Either;

impl<'de> Deserialize<'de> for Value2 {
    #[expect(private_interfaces)]
    fn deserialize_with<R: Source<'de>>(der: &mut Deserializer<R>, _: PrivateMethod) -> ResultKind<Self> {
        let range_component = 'non_range: {
            let val = match der.src.begin(&mut der.buf)? {
                Indicator::Unit => Self::Unit,
                Indicator::Bool(b) => Self::Bool(b),
                Indicator::Char(ch) => break 'non_range Either::Left(Scalar::Char(ch)),
                Indicator::Byte(byte) => break 'non_range Either::Left(Scalar::Number(byte.into())),
                Indicator::Number(kind) => break 'non_range Either::Left(Scalar::Number(der.src.parse_number(kind)?)),
                Indicator::String(kind) => Self::String(der.src.parse_string(kind, &mut der.buf)?.either_into()),
                Indicator::Bytes(kind) => Self::ByteBuf(der.src.parse_bytes(kind, &mut der.buf)?.either_into()),
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
                    Initiator::DotDot => {
                        break 'non_range Either::Right(RangeSeparator::DotDot);
                    }
                    Initiator::DotDotEq => {
                        break 'non_range Either::Right(RangeSeparator::DotDotEq);
                    }
                },
                Indicator::NominalPath(path) => match der.src.nominal_body_initiator()? {
                    Some(init) => match init {
                        NominalBodyInitiator::Tuple => {
                            let val = Self::TupleStruct(Box::new((path.into(), deserialize_values(der)?)));
                            der.src.end_tuple()?;
                            val
                        }
                        NominalBodyInitiator::Struct => {
                            let val = Self::MapStruct(Box::new((path.into(), deserialize_fields_map(der)?)));
                            der.src.end_map_like()?;
                            val
                        }
                    },
                    None => {
                        der.src.adjacent_to_delim_expected(ErrorKind::InvalidNominalBody)?;
                        Self::UnitStruct(Box::new(path.into()))
                    }
                },
                Indicator::ExplicitNewtype(ident) => {
                    Self::Newtype(Box::new((ident.to_owned(), deserialize_value(der)?)))
                }
            };
            return Ok(val);
        };
        let val = match range_component {
            Either::Left(scalar) => {
                if let Some(sep) = der.src.range_separator()? {
                    if der.src.adjacent_to_scalar()? {
                        (match sep {
                            RangeSeparator::DotDot => Self::Range,
                            RangeSeparator::DotDotEq => Self::RangeInclusive,
                        })(Box::new((scalar, deserialize_scalar(der)?)))
                    } else if let RangeSeparator::DotDot = sep {
                        Self::RangeFrom(Box::new(scalar))
                    } else {
                        return Err(ErrorKind::UnexpectedRangeDotDotEq);
                    }
                } else {
                    scalar.into()
                }
            }
            Either::Right(sep) => {
                if der.src.adjacent_to_scalar()? {
                    (match sep {
                        RangeSeparator::DotDot => Self::RangeTo,
                        RangeSeparator::DotDotEq => Self::RangeToInclusive,
                    })(Box::new(deserialize_scalar(der)?))
                } else if let RangeSeparator::DotDot = sep {
                    Self::RangeFull
                } else {
                    return Err(ErrorKind::UnexpectedRangeDotDotEq);
                }
            }
        };
        Ok(val)
    }
}

fn deserialize_scalar<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<Scalar> {
    let scalar = match der.src.begin(&mut der.buf)? {
        Indicator::Char(ch) => Scalar::Char(ch),
        Indicator::Byte(byte) => Scalar::Number(byte.into()),
        Indicator::Number(kind) => Scalar::Number(der.src.parse_number(kind)?),
        _ => return Err(ErrorKind::ExpectedScalar),
    };
    Ok(scalar)
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
        let field = Ident::new_unchecked(der.src.parse_identifier(&mut der.buf)?).to_owned();
        der.src
            .delim_expected(Delimiter::FatArrow, ErrorKind::ExpectedFatArrow)?;
        let value = Value2::deserialize_with(der, PrivateMethod)?;
        der.src.delim(Delimiter::Comma)?;
        fields_map.insert(field, value);
    }
    der.ttl_leave();
    Ok(fields_map)
}
