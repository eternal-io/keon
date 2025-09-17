use super::*;
use core::str::FromStr;

pub fn parse_value(s: &str) -> Result<Value> {
    Value::from_str(s)
}

pub fn parse_values(s: &str) -> ValueIterParser<'_> {
    ValueIterParser { der: Parser::new(s) }
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
        todo!()
    }
}
