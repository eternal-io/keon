use super::*;

impl<'de> Parsable<'de> for Value {
    #[inline]
    fn parse_limited_via(der: &mut Parser<'de>, limit: Option<u32>) -> Result<Self> {
        Value::deserialize(der, limit)
    }
}

impl<'de> Parser<'de> {
    #[inline]
    fn recursion_guard(&mut self, ttl: Option<u32>) -> Result<Option<u32>> {
        match ttl {
            None => Ok(None),
            Some(ttl) => match ttl.checked_sub(1) {
                Some(ttl) => Ok(Some(ttl)),
                None => self.raise(ErrorKind::ExceededRecursionLimit),
            },
        }
    }
}

//------------------------------------------------------------------------------

struct Unspecified;

impl MakeNum for Unspecified {
    type Output = Value;

    fn make_num(start: usize, slice: &[u8], kind: NumKind, typ: Option<NumType>) -> Result<Self::Output> {
        let num = match typ {
            Some(typ) => match typ {
                NumType::U8 => u8::make_num(start, slice, kind, None)?.into(),
                NumType::U16 => u16::make_num(start, slice, kind, None)?.into(),
                NumType::U32 => u32::make_num(start, slice, kind, None)?.into(),
                NumType::U64 => u64::make_num(start, slice, kind, None)?.into(),
                NumType::U128 => u128::make_num(start, slice, kind, None)?.into(),
                NumType::I8 => i8::make_num(start, slice, kind, None)?.into(),
                NumType::I16 => i16::make_num(start, slice, kind, None)?.into(),
                NumType::I32 => i32::make_num(start, slice, kind, None)?.into(),
                NumType::I64 => i64::make_num(start, slice, kind, None)?.into(),
                NumType::I128 => i128::make_num(start, slice, kind, None)?.into(),
                NumType::F32 => f32::make_num(start, slice, kind, None)?.into(),
                NumType::F64 => f64::make_num(start, slice, kind, None)?.into(),
            },
            None => Value::NumberNoSuffix(if let NumKind::Float = kind {
                f64::make_num(start, slice, kind, None)?.into()
            } else if let Some(b'-') = slice.first() {
                i64::make_num(start, slice, kind, None)?.into()
            } else {
                u64::make_num(start, slice, kind, None)?.into()
            }),
        };

        Ok(num)
    }

    fn make_special(_start: usize, special: NumSpecial) -> Result<Self::Output> {
        let special = match special {
            NumSpecial::Infinity => f64::INFINITY,
            NumSpecial::NegInfinity => f64::NEG_INFINITY,
            NumSpecial::NotANumber => f64::NAN,
        };

        Ok(Value::NumberNoSuffix(special.into()))
    }
}

//------------------------------------------------------------------------------

impl Value {
    fn deserialize(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        if der.is_corrupted() {
            return der.raise(ErrorKind::Corrupted);
        }

        ttl = der.recursion_guard(ttl)?;

        der.consume_whitespace_comment_first()?;

        let start = der.pos;
        let val = match der.lookahead()? {
            Kind::_Char => der.parse_char()?.into(),
            Kind::_Byte => der.parse_byte()?.into(),
            Kind::_Bytes => der.parse_byte_string()?.converge(),
            Kind::_StringOrParagraph => der.parse_string_or_paragraph()?.converge(),

            Kind::Bool(v) => v.into(),
            Kind::Number(was_special) => der.parse_number_with_known::<Unspecified>(start, was_special)?,

            Kind::Maybe => Self::deserialize_maybe(der, ttl)?,
            Kind::Tuple => Self::deserialize_tuple(der, ttl)?,
            Kind::Seq => Self::deserialize_seq(der, ttl)?,
            Kind::Map => Self::deserialize_map(der, ttl)?,

            kind @ (Kind::NominalUnnamed | Kind::NominalStemOnly { .. } | Kind::NominalFullNamed { .. }) => {
                Self::deserialize_nominal(der, ttl, kind.into())?
            }
        };

        Ok(val)
    }

    //------------------------------------------------------------------------------

    /// NOTE: The leading `?` has already been consumed.
    fn deserialize_maybe(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        ttl = der.recursion_guard(ttl)?;

        let val = match der.adjacent_to_delim() {
            true => None,
            false => Some(Box::new(Self::deserialize(der, ttl)?)),
        };

        Ok(Value::Maybe(val))
    }

    /// NOTE: The leading `(` has already been consumed.
    fn deserialize_tuple(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        ttl = der.recursion_guard(ttl)?;

        let mut seq = Values::new();
        while !der.adjacent_to_delim() {
            seq.push(Self::deserialize(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_(")")? {
            return der.raise(ErrorKind::ExpectedTupleClose);
        }

        Ok(Value::Tuple((!seq.is_empty()).then(|| Box::new(seq))))
    }

    /// NOTE: The leading `[` has already been consumed.
    fn deserialize_seq(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        ttl = der.recursion_guard(ttl)?;

        let mut seq = Values::new();
        while !der.adjacent_to_delim() {
            seq.push(Self::deserialize(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_("]")? {
            return der.raise(ErrorKind::ExpectedSequenceClose);
        }

        Ok(Value::Seq(Box::new(seq)))
    }

    /// NOTE: The leading `{` has already been consumed.
    fn deserialize_map(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        ttl = der.recursion_guard(ttl)?;

        let mut map = ValueMap::new();
        while !der.adjacent_to_delim() {
            let key = Self::deserialize(der, ttl)?;
            if !der.consume_ws_("=>")? {
                return der.raise(ErrorKind::ExpectedFatArrow);
            }

            map.insert(key, Self::deserialize(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_("}")? {
            return der.raise(ErrorKind::ExpectedBraceClose);
        }

        Ok(Value::Map(Box::new(map)))
    }

    //------------------------------------------------------------------------------

    /// NOTE: The leading nominal path has already been consumed.
    fn deserialize_nominal(der: &mut Parser, ttl: Option<u32>, mut prototype: Nominal) -> Result<Value> {
        let nom = match der.nominal_lookahead()? {
            NominalKind::Unit => prototype,

            NominalKind::Tuple => {
                prototype.set_struct(Self::deserialize_nominal_tuple(der, ttl)?);
                prototype
            }

            NominalKind::Struct => {
                prototype.set_struct(Self::deserialize_nominal_struct(der, ttl)?);
                prototype
            }
        };

        Ok(Value::Nominal(Box::new(nom)))
    }

    /// NOTE: The leading `(` has been consumed.
    fn deserialize_nominal_tuple(der: &mut Parser, mut ttl: Option<u32>) -> Result<NominalValue> {
        ttl = der.recursion_guard(ttl)?;

        let mut seq = Values::new();
        while !der.adjacent_to_delim() {
            seq.push(Self::deserialize(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_("]")? {
            return der.raise(ErrorKind::ExpectedSequenceClose);
        }

        Ok(NominalValue::Tuple(seq))
    }

    /// NOTE: The leading `{` has been consumed.
    fn deserialize_nominal_struct(der: &mut Parser, mut ttl: Option<u32>) -> Result<NominalValue> {
        ttl = der.recursion_guard(ttl)?;

        let mut map = alloc::collections::BTreeMap::new();
        while !der.adjacent_to_delim() {
            let key = der.consume_ident()?.into();
            if !der.consume_ws_(":")? {
                return der.raise(ErrorKind::ExpectedColon);
            }

            map.insert(key, Self::deserialize(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_("}")? {
            return der.raise(ErrorKind::ExpectedBraceClose);
        }

        Ok(NominalValue::Struct(map))
    }
}

//------------------------------------------------------------------------------

impl From<Kind> for Nominal {
    #[inline]
    fn from(value: Kind) -> Nominal {
        match value {
            Kind::NominalUnnamed => Nominal::Unnamed {
                stru: NominalValue::Unit,
            },

            Kind::NominalStemOnly { name } => Nominal::StemOnly {
                stru: NominalValue::Unit,
                name,
            },

            Kind::NominalFullNamed { name, parent } => Nominal::FullNamed {
                stru: NominalValue::Unit,
                name,
                parent,
            },

            _ => panic!(),
        }
    }
}

//------------------------------------------------------------------------------

/// NOTE: A name starting with an underscore indicates that
/// the parser does not consume any characters during the lookahead.
enum Kind {
    _Char,
    _Byte,
    _Bytes,
    _StringOrParagraph,
    Bool(bool),
    Number(Option<NumSpecial>),
    Maybe,
    Tuple,
    Seq,
    Map,
    NominalUnnamed,
    NominalStemOnly { name: Str },
    NominalFullNamed { name: Str, parent: Str },
}

enum NominalKind {
    Unit,
    Tuple,
    Struct,
}

impl Parser<'_> {
    #[inline]
    fn lookahead(&mut self) -> Result<Kind> {
        if self.consume_ws_("?")? {
            return Ok(Kind::Maybe);
        } else if self.consume_ws_("(")? {
            return Ok(Kind::Tuple);
        } else if self.consume_ws_("[")? {
            return Ok(Kind::Seq);
        } else if self.consume_ws_("{")? {
            return Ok(Kind::Map);
        }

        let mut num_special = 'non_number: {
            let kind = match self.rest_bytes() {
                [b'\'', ..] => Kind::_Char,

                [b'b', b'\'', ..] => Kind::_Byte,

                [b'b', b'"' | b'`', ..]
                | [b'b', b'1', b'6', b'"', ..]
                | [b'b', b'3', b'2', b'"', ..]
                | [b'b', b'6', b'4', b'"', ..] => Kind::_Bytes,

                [b'"', ..] | [b'`', b'`' | b'"' | b'|', ..] => Kind::_StringOrParagraph,

                [b'-' | b'0'..=b'9' | b'.', ..] => break 'non_number None,

                [_, ..] => match self.consume_keyword_or_ident_or_underscore()? {
                    Token::Keyword(kw) => match kw {
                        Keyword::True => Kind::Bool(true),
                        Keyword::False => Kind::Bool(false),
                        Keyword::Infinity => break 'non_number Some(NumSpecial::Infinity),
                        Keyword::NotANumber => break 'non_number Some(NumSpecial::NotANumber),
                    },

                    Token::Identifier(name) => {
                        if self.consume_ws_("::")? {
                            let parent = name.into();
                            let name = self.consume_ident()?.into();

                            Kind::NominalFullNamed { name, parent }
                        } else {
                            Kind::NominalStemOnly { name: name.into() }
                        }
                    }

                    Token::Underscore => {
                        if self.consume_ws_("::")? {
                            let name = self.consume_ident()?.into();

                            Kind::NominalStemOnly { name }
                        } else {
                            Kind::NominalUnnamed
                        }
                    }
                },

                [] => return self.raise(ErrorKind::ExpectedValue),
            };

            return Ok(kind);
        };

        if num_special.is_none() {
            if self.consume_ws_("-inf")? {
                num_special = Some(NumSpecial::NegInfinity);
            } else if self.consume_ws_("-NaN")? {
                num_special = Some(NumSpecial::NotANumber);
            }
        }

        Ok(Kind::Number(num_special))
    }

    #[inline]
    fn nominal_lookahead(&mut self) -> Result<NominalKind> {
        if self.adjacent_to_delim() {
            Ok(NominalKind::Unit)
        } else if self.consume_ws_("(")? {
            Ok(NominalKind::Tuple)
        } else if self.consume_ws_("{")? {
            Ok(NominalKind::Struct)
        } else {
            self.raise(ErrorKind::ExpectedNominalValue)
        }
    }
}
