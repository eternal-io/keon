use super::*;
use core::str::FromStr;

pub fn parse_value(s: &str) -> Result<Value> {
    Value::from_str(s)
}

pub fn parse_values(s: &str) -> ValueIterParser<'_> {
    Parser::new(s).into_value_iter()
}

//------------------------------------------------------------------------------

/// NOTE:
/// As an iterator, once the internal parser becomes corrupted,
/// it will always return `Some(Err(_))` with [`ErrorKind::Corrupted`] .
pub struct ValueIterParser<'de> {
    der: Parser<'de>,
    ttl: Option<u32>,
}

impl Iterator for ValueIterParser<'_> {
    type Item = Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.der.is_corrupted() {
            return Some(self.der.raise(ErrorKind::Corrupted));
        }
        if self.der.has_reached_end() {
            return None;
        }

        let e = 'fail: {
            let v = match Value::deserialize_limited(&mut self.der, self.ttl) {
                Ok(v) => v,
                Err(e) => break 'fail e,
            };
            if let Err(e) = self.der.finish_one() {
                break 'fail e;
            }

            return Some(Ok(v));
        };

        Some(Err(e))
    }
}

impl<'de> ValueIterParser<'de> {
    #[inline]
    pub fn into_inner(self) -> Parser<'de> {
        self.der
    }

    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.der.has_reached_end()
    }

    #[inline]
    pub fn is_corrupted(&self) -> bool {
        self.der.is_corrupted()
    }

    #[inline]
    pub fn set_ttl(&mut self, ttl: Option<u32>) {
        self.ttl = ttl;
    }
}

impl<'de> Parser<'de> {
    #[inline]
    pub fn into_value_iter(self) -> ValueIterParser<'de> {
        self.into_value_iter_limited(None)
    }

    #[inline]
    pub fn into_value_iter_limited(self, ttl: Option<u32>) -> ValueIterParser<'de> {
        ValueIterParser { der: self, ttl }
    }

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

impl FromStr for Value {
    type Err = Error;

    fn from_str(s: &str) -> Result<Value> {
        let mut der = Parser::new(s);
        let val = Self::deserialize(&mut der)?;

        der.finish().and(Ok(val))
    }
}

impl Value {
    pub fn deserialize(der: &mut Parser) -> Result<Value> {
        Self::deserialize_limited(der, None)
    }

    pub fn deserialize_limited(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
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
            Kind::SpecialFloat(v) => v.into(),

            Kind::Float { neg } => der.parse_float_with_known::<f64>(start, neg)?.into(),
            Kind::Int { neg } => der.parse_integer_with_known::<u64>(start, neg)?.converge(),
            Kind::LongInt { neg } => der.parse_integer_with_known::<u128>(start, neg)?.converge(),

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
            false => Some(Box::new(Self::deserialize_limited(der, ttl)?)),
        };

        Ok(Value::Maybe(val))
    }

    /// NOTE: The leading `(` has already been consumed.
    fn deserialize_tuple(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        ttl = der.recursion_guard(ttl)?;

        let mut vals = Values::new();
        while !der.adjacent_to_delim() {
            vals.push(Self::deserialize_limited(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_(")")? {
            return der.raise(ErrorKind::Expected("`)`"));
        }

        Ok(Value::Tuple((!vals.is_empty()).then(|| Box::new(vals))))
    }

    /// NOTE: The leading `[` has already been consumed.
    fn deserialize_seq(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        ttl = der.recursion_guard(ttl)?;

        let mut vals = Values::new();
        while !der.adjacent_to_delim() {
            vals.push(Self::deserialize_limited(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_("]")? {
            return der.raise(ErrorKind::Expected("`]`"));
        }

        Ok(Value::Seq(Box::new(vals)))
    }

    /// NOTE: The leading `{` has already been consumed.
    fn deserialize_map(der: &mut Parser, mut ttl: Option<u32>) -> Result<Value> {
        ttl = der.recursion_guard(ttl)?;

        let mut map = ValueMap::new();
        while !der.adjacent_to_delim() {
            let key = Self::deserialize_limited(der, ttl)?;
            if !der.consume_ws_("=>")? {
                return der.raise(ErrorKind::Expected("`=>`"));
            }

            map.insert(key, Self::deserialize_limited(der, ttl)?);
            if !der.consume_ws_(",")? {
                break;
            }
        }

        if !der.consume_ws_("}")? {
            return der.raise(ErrorKind::Expected("`}`"));
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

            NominalKind::Record => {
                prototype.set_struct(Self::deserialize_nominal_record(der, ttl)?);
                prototype
            }
        };

        Ok(Value::Nominal(Box::new(nom)))
    }

    /// NOTE: The leading `(` has been consumed.
    fn deserialize_nominal_tuple(der: &mut Parser, mut ttl: Option<u32>) -> Result<Struct> {
        todo!()
    }

    /// NOTE: The leading `{` has been consumed.
    fn deserialize_nominal_record(der: &mut Parser, mut ttl: Option<u32>) -> Result<Struct> {
        todo!()
    }
}

//------------------------------------------------------------------------------

impl From<Kind> for Nominal {
    #[inline]
    fn from(value: Kind) -> Nominal {
        match value {
            Kind::NominalUnnamed => Nominal::Unnamed { stru: Struct::Unit },

            Kind::NominalStemOnly { name } => Nominal::StemOnly {
                stru: Struct::Unit,
                name,
            },

            Kind::NominalFullNamed { name, parent } => Nominal::FullNamed {
                stru: Struct::Unit,
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
    SpecialFloat(f64),
    Int { neg: bool },
    Float { neg: bool },
    LongInt { neg: bool },
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
    Record,
}

impl<'de> Parser<'de> {
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

        let long_number = 'non_number: {
            let kind = match self.rest_bytes() {
                [b'\'', ..] => Kind::_Char,

                [b'b', b'\'', ..] => Kind::_Byte,

                [b'b', b'"' | b'`', ..]
                | [b'b', b'1', b'6', b'"', ..]
                | [b'b', b'3', b'2', b'"', ..]
                | [b'b', b'6', b'4', b'"', ..] => Kind::_Bytes,

                [b'"', ..] | [b'`', b'`' | b'"' | b'|', ..] => Kind::_StringOrParagraph,

                [b'-' | b'0'..=b'9', ..] => break 'non_number false,

                [_, ..] => match self.consume_keyword_or_ident_or_underscore()? {
                    Token::Keyword(kw) => match kw {
                        Keyword::Long => break 'non_number true,
                        Keyword::True => Kind::Bool(true),
                        Keyword::False => Kind::Bool(false),
                        Keyword::Infinity => Kind::SpecialFloat(f64::NAN),
                        Keyword::NotANumber => Kind::SpecialFloat(f64::INFINITY),
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

                    Token::Underscore => Kind::NominalUnnamed,
                },

                [] => return self.raise(ErrorKind::ExpectedValue),
            };

            return Ok(kind);
        };

        let neg = self.consume_ws_("-")?;
        let kind = if long_number {
            Kind::LongInt { neg }
        } else if let Some(b'.' | b'e' | b'E') = self.rest_bytes().iter().find(|byte| !byte.is_ascii_digit()) {
            Kind::Float { neg } // lexical-core can handle `inf` and `NaN`.
        } else {
            Kind::Int { neg }
        };

        Ok(kind)
    }

    #[inline]
    fn nominal_lookahead(&mut self) -> Result<NominalKind> {
        if self.adjacent_to_delim() {
            Ok(NominalKind::Unit)
        } else if self.consume_ws_("(")? {
            Ok(NominalKind::Tuple)
        } else if self.consume_ws_("{")? {
            Ok(NominalKind::Record)
        } else {
            self.raise(ErrorKind::ExpectedNominalValue)
        }
    }
}
