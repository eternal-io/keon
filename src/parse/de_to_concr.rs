use super::*;
use chumsky::prelude::*;
use serde::{de::Visitor, Deserialize};

type Err = extra::Err<Error>;

pub fn parse<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T> {
    let mut der = Deserializer::new(s);
    let value = T::deserialize(&mut der)?;

    der.finish()?.then_some(value).ok_or(Error::new(ErrorKind::ExpectedEof))
}

// pub fn parse_many<'de, T: Deserialize<'de>>(s: &'de str) -> Result<Vec<T>> {
//     let mut der = Deserializer::new(s);
//     let value = T::deserialize(&mut der)?;
//     der.finish()?;
//     Ok(value)
// }

pub struct Deserializer<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Deserializer<'a> {
    #[inline]
    fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    #[inline]
    fn bump(&mut self, n: usize) {
        self.offset += n;
        debug_assert!(self.source.is_char_boundary(self.offset));
    }

    #[inline]
    fn rest(&self) -> &str {
        self.source.split_at(self.offset).1
    }

    #[inline]
    pub fn finish(&mut self) -> Result<bool> {
        // let delim: _ = text::whitespace::<_, extra::Default>();

        // delim.parse(self.rest()).into_result().or()
        // self.rest()
        //     .chars()
        //     .all(char::is_whitespace)
        //     .then_some(())
        //     .ok_or(Error::new(ErrorKind::ExpectedEof))

        todo!()
    }
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    #![allow(unused_variables)]
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        Error::raise(ErrorKind::WontImplement)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_i8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_i64(vis)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_i64(vis)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.deserialize_i64(vis)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_u8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_u16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_u32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_u64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_f32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_f64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_char<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_str<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_string<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_option<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_unit<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_seq<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, len: usize, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_map<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        vis: V,
    ) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        todo!()
    }
}
