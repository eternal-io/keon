use super::{error::*, source::*, Deserialize, Deserializer, PrivateMethod};
use crate::value::*;
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
                Indicator::Number(kind) => break 'non_range Either::Left(Scalar::Number(der.parse_number(kind)?)),
                Indicator::String(kind) => Self::String(der.src.parse_string(kind, &mut der.buf)?.either_into()),
                Indicator::Bytes(kind) => Self::ByteBuf(der.src.parse_bytes(kind, &mut der.buf)?.either_into()),
                Indicator::Initiator(init) => match init {
                    Initiator::Maybe => Self::Maybe(if !der.adjacent_to_delim()? {
                        Some(Box::new(deserialize_value(der)?))
                    } else {
                        None
                    }),
                    Initiator::Array => {
                        let val = Self::Array(Box::new(deserialize_values(der)?));
                        der.end_array()?;
                        val
                    }
                    Initiator::Tuple => {
                        let val = Self::Tuple(Box::new(deserialize_values(der)?));
                        der.end_tuple()?;
                        val
                    }
                    Initiator::MapLike => {
                        let val = Self::Map(Box::new(deserialize_values_map(der)?));
                        der.end_map_like()?;
                        val
                    }
                    Initiator::DotDot => {
                        break 'non_range Either::Right(RangeSeparator::DotDot);
                    }
                    Initiator::DotDotEq => {
                        break 'non_range Either::Right(RangeSeparator::DotDotEq);
                    }
                },
                Indicator::Identifier(name) => {
                    let name = name.map(ToOwned::to_owned);
                    match der.nominal_body_initiator()? {
                        Some(init) => match init {
                            NominalBodyInitiator::Tuple => {
                                let body = deserialize_values(der)?;
                                der.end_tuple()?;
                                Self::TupleStruct(Box::new(Struct { name, body }))
                            }
                            NominalBodyInitiator::Struct => {
                                let body = deserialize_fields_map(der)?;
                                der.end_map_like()?;
                                Self::MapStruct(Box::new(Struct { name, body }))
                            }
                        },
                        None => {
                            der.adjacent_to_delim_expected(ErrorKind::InvalidNominalBody)?;
                            Self::UnitStruct(name.map(Box::new))
                        }
                    }
                }
                Indicator::ExplicitNewtype(name) => {
                    let name = name.map(ToOwned::to_owned);
                    let body = deserialize_value(der)?;
                    Self::Newtype(Box::new(Struct { name, body }))
                }
                Indicator::ExplicitVariant(name, variant) => {
                    let name = name.map(ToOwned::to_owned);
                    let variant = variant.to_owned();
                    match der.nominal_body_initiator()? {
                        Some(init) => match init {
                            NominalBodyInitiator::Tuple => {
                                let body = deserialize_values(der)?;
                                der.end_tuple()?;
                                Self::TupleVariant(Box::new(Variant { name, variant, body }))
                            }
                            NominalBodyInitiator::Struct => {
                                let body = deserialize_fields_map(der)?;
                                der.end_map_like()?;
                                Self::MapVariant(Box::new(Variant { name, variant, body }))
                            }
                        },
                        None => {
                            der.adjacent_to_delim_expected(ErrorKind::InvalidNominalBody)?;
                            Self::UnitVariant(Box::new(Variant {
                                name,
                                variant,
                                body: (),
                            }))
                        }
                    }
                }
            };
            return Ok(val);
        };
        let val = match range_component {
            Either::Left(scalar) => {
                if let Some(sep) = der.range_separator()? {
                    if der.adjacent_to_scalar()? {
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
                if der.adjacent_to_scalar()? {
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
        Indicator::Number(kind) => Scalar::Number(der.parse_number(kind)?),
        _ => return Err(ErrorKind::ExpectedScalar),
    };
    Ok(scalar)
}

fn deserialize_value<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<Value2> {
    der.enter_nesting()?;
    let value = Value2::deserialize_with(der, PrivateMethod)?;
    der.exit_nesting();
    Ok(value)
}

fn deserialize_values<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<Values2> {
    der.enter_nesting()?;
    let mut values = Values2::new();
    loop {
        if der.adjacent_to_delim()? {
            break;
        }
        let value = Value2::deserialize_with(der, PrivateMethod)?;
        der.delim(Delimiter::Comma)?;
        values.push(value);
    }
    der.exit_nesting();
    Ok(values)
}

fn deserialize_values_map<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<ValuesMap2> {
    der.enter_nesting()?;
    let mut values_map = ValuesMap2::new();
    loop {
        if der.adjacent_to_delim()? {
            break;
        }
        let key = Value2::deserialize_with(der, PrivateMethod)?;
        der.delim_expected(Delimiter::Colon, ErrorKind::ExpectedColon)?;
        let value = Value2::deserialize_with(der, PrivateMethod)?;
        der.delim(Delimiter::Comma)?;
        values_map.insert(key, value);
    }
    der.exit_nesting();
    Ok(values_map)
}

fn deserialize_fields_map<'de, R: Source<'de>>(der: &mut Deserializer<R>) -> ResultKind<FieldsMap2> {
    der.enter_nesting()?;
    let mut fields_map = FieldsMap2::new();
    loop {
        if der.adjacent_to_delim()? {
            break;
        }
        let field = Ident::new_unchecked(der.src.parse_identifier(&mut der.buf)?).to_owned();
        der.delim_expected(Delimiter::FatArrow, ErrorKind::ExpectedFatArrow)?;
        let value = Value2::deserialize_with(der, PrivateMethod)?;
        der.delim(Delimiter::Comma)?;
        fields_map.insert(field, value);
    }
    der.exit_nesting();
    Ok(fields_map)
}
