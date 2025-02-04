use super::*;
use core::num::NonZeroU32;
use kaparser::*;
use serde::de::{
    value::{EnumAccessDeserializer, StrDeserializer},
    DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use smol_str::SmolStr;

pub struct Deserializer<'de, R: Read> {
    par: Utf8Parser<'de, R>,
    ttl: usize,
    line_off: usize,
    line_ctr: usize,
}

impl<'de> Deserializer<'de, Slice> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(slice: &'de str) -> Self {
        Self::new(Utf8Parser::from_str(slice))
    }

    pub fn from_bytes(bytes: &'de [u8]) -> Result<Self> {
        Ok(Self::new(Utf8Parser::from_bytes(bytes)?))
    }
}

impl<'de, R: Read> Deserializer<'de, R> {
    pub fn from_reader(reader: R) -> Self {
        Self::new(Utf8Parser::from_reader(reader))
    }

    pub fn from_parser(parser: Utf8Parser<'de, R>) -> Self {
        Self::new(parser)
    }

    pub fn into_parser(self) -> Utf8Parser<'de, R> {
        self.par
    }

    fn new(par: Utf8Parser<'de, R>) -> Self {
        Self {
            par,
            ttl: RECURSION_LIMIT,
            line_off: 0,
            line_ctr: 0,
        }
    }

    fn raise_error<T>(&self, kind: ErrorKind) -> Result<T> {
        todo!()
    }

    //------------------------------------------------------------------------------

    const PR_WS: &'static [char] = &['\n', '\t', '\r', '\x0b', '\x0c', '\x20'];
    const PR_WS_NN: &'static [char] = &['\t', '\r', '\x0b', '\x0c', '\x20'];

    fn meet_newline(&mut self) {
        self.par.bump(1);
        self.line_off = self.par.consumed();
        self.line_ctr += 1;
    }

    fn after_whitespace(&mut self) -> Result<Option<char>> {
        Ok(loop {
            match self.par.take_while(Self::PR_WS_NN)?.1 {
                None => break None,
                Some(ch) => match ch {
                    '\n' => self.meet_newline(),
                    _ if Self::PR_WS.contains(&ch) => continue,
                    _ => break Some(ch),
                },
            }
        })
    }

    //------------------------------------------------------------------------------

    fn parse<'i, V: Visitor<'de>>(&'i mut self, vis: V) -> Result<V::Value> {
        todo!()
    }
}

impl<'de, R: Read> serde::Deserializer<'de> for &mut Deserializer<'de, R> {
    type Error = Error;
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let (ttl, overflowed) = self.ttl.overflowing_sub(1);
        if overflowed {
            self.raise_error(ErrorKind::ExceededRecursionLimit)?
        }

        self.ttl = ttl;

        let val = self.parse(vis);

        self.ttl += 1;

        val
    }
}
