use super::*;
use chumsky::prelude::*;
use serde::{de::Visitor, Deserialize};

type Err = extra::Err<Error>;

pub fn parse<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T> {
    let mut der = Deserializer::new(s)?;
    let value = T::deserialize(&mut der)?;
    der.finish().and(Ok(value))
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
    pub fn new(source: &'a str) -> Result<Self> {
        let mut der = Self { source, offset: 0 };
        der.consume_whitespace_comment()?;
        Ok(der)
    }

    #[inline]
    pub fn has_reached_end(&self) -> bool {
        self.rest().is_empty()
    }

    #[inline]
    pub fn finish(&mut self) -> Result<()> {
        self.consume_ws_(";")?;

        if self.has_reached_end() {
            Ok(())
        } else {
            Error::raise(ErrorKind::ExpectedEnd)
        }
    }

    #[inline]
    pub fn finish_one(&mut self) -> Result<bool> {
        if self.consume_ws_(";")? {
            Ok(self.has_reached_end())
        } else if self.has_reached_end() {
            Ok(true)
        } else {
            Error::raise(ErrorKind::ExpectedSemiOrEnd)
        }
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
    fn consume(&mut self, pat: &'static str) -> bool {
        match self.rest().starts_with(pat) {
            false => false,
            true => {
                self.bump(pat.len());
                true
            }
        }
    }

    #[inline]
    fn consume_ws_(&mut self, pat: &'static str) -> Result<bool> {
        Ok(match self.rest().starts_with(pat) {
            false => false,
            true => {
                self.bump(pat.len());
                self.consume_whitespace_comment()?;
                true
            }
        })
    }

    #[inline]
    fn consume_while(&mut self, pred: impl FnMut(&char) -> bool) {
        self.bump(self.rest().chars().take_while(pred).count());
    }

    #[inline]
    fn consume_whitespace_comment(&mut self) -> Result<()> {
        loop {
            self.consume_while(char::is_ascii_whitespace);

            if self.consume("/*") {
                let mut depth = 1u8;

                while depth != 0 {
                    self.consume_while(|ch| *ch != '/');

                    if let Some(b'*') = self.source.as_bytes().get(self.offset - 1) {
                        self.bump(1);
                        depth -= 1;
                    } else if self.consume("/*") {
                        depth += 1;

                        if depth == u8::MAX {
                            return Error::raise(ErrorKind::DeeplyNestedComment);
                        }
                    }

                    if self.has_reached_end() {
                        return Error::raise(ErrorKind::UnclosedComment);
                    }
                }
            } else {
                break;
            }
        }

        self.consume_while(char::is_ascii_whitespace);

        Ok(())
    }
}

macro_rules! deserialize_num {
    ( $self:ident, $ty:ty, $visitor:ident, $method:ident ) => {{
        let (x, o) = lexical_core::parse_partial::<$ty>($self.rest().as_bytes())
            .map_err(ErrorKind::InvalidNumber)
            .map_err(Error::new)?;
        $self.bump(o);
        $visitor.$method(x)
    }};
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    #![allow(unused_variables)]
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        Error::raise(ErrorKind::WontImplement)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        if self.consume_ws_("true")? {
            vis.visit_bool(true)
        } else if self.consume_ws_("false")? {
            vis.visit_bool(false)
        } else {
            Error::raise(ErrorKind::ExpectedBoolean)
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, i8, vis, visit_i8)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, i16, vis, visit_i16)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, i32, vis, visit_i32)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, i64, vis, visit_i64)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, i128, vis, visit_i128)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, u8, vis, visit_u8)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, u16, vis, visit_u16)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, u32, vis, visit_u32)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, u64, vis, visit_u64)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, u128, vis, visit_u128)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, f32, vis, visit_f32)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        deserialize_num!(self, f64, vis, visit_f64)
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
