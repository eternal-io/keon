use super::*;
use core::str::FromStr;

pub fn parse_value(s: &str) -> Result<Value> {
    Value::from_str(s)
}

pub fn parse_many_value(s: &str) -> IterValueParser<'_> {
    IterValueParser { der: Parser::new(s) }
}

//------------------------------------------------------------------------------

pub struct IterValueParser<'de> {
    der: Parser<'de>,
}

impl Iterator for IterValueParser<'_> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

//------------------------------------------------------------------------------

impl FromStr for Value {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut der = Parser::new(s);
        Self::parse(&mut der)
    }
}

impl Value {
    pub fn parse(der: &mut Parser<'_>) -> Result<Self> {
        todo!()
    }
}
