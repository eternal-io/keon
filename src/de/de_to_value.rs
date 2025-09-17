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
            let v = match Value::deserialize(&mut self.der) {
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
}

impl<'de> Parser<'de> {
    #[inline]
    pub fn into_value_iter(self) -> ValueIterParser<'de> {
        ValueIterParser { der: self }
    }
}

//------------------------------------------------------------------------------

impl FromStr for Value {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut der = Parser::new(s);
        let val = Self::deserialize(&mut der)?;

        der.finish().and(Ok(val))
    }
}

impl Value {
    pub fn deserialize(der: &mut Parser<'_>) -> Result<Self> {
        let val = match der.lookahead()? {
            Kind::_Char => der.parse_char()?.into(),
            Kind::_Byte => der.parse_byte()?.into(),
            Kind::_Bytes => der.parse_byte_string()?.converge(),
            Kind::_StringOrParagraph => der.parse_string_or_paragraph()?.converge(),

            Kind::Bool(v) => v.into(),
            Kind::SpecialFloat(v) => v.into(),

            Kind::Int { neg } => todo!(),
            Kind::Float { neg } => todo!(),
            Kind::LongInt { neg } => todo!(),

            Kind::Maybe => Self::deserialize_maybe(der)?,
            Kind::Tuple => todo!(),
            Kind::Seq => todo!(),
            Kind::Map => todo!(),

            Kind::NominalUnnamed => todo!(),
            Kind::NominalStemOnly { name } => todo!(),
            Kind::NominalFullNamed { name, parent } => todo!(),
        };

        Ok(val)
    }

    /// NOTE: The leading `?` has already been consumed.
    fn deserialize_maybe(der: &mut Parser) -> Result<Self> {
        let val = if der.adjacent_to_delim() {
            Value::Maybe(None)
        } else {
            Value::Maybe(Some(Box::new(Self::deserialize(der)?)))
        };

        Ok(val)
    }
}
