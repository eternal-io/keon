use self::{error::*, source::*};
use crate::{format::*, value::*};
use core::{cmp::Ordering, marker::PhantomData};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use either::Either;

mod de_to_concr;
mod de_to_value;
pub mod error;
pub mod source;

pub const DEFAULT_RECURSION_LIMIT: isize = 160;

pub trait Deserialize<'de>: Sized {
    fn deserialize_with<R: Source<'de>>(der: &mut Deserializer<R>) -> Result<Self>;
}

pub struct Deserializer<R> {
    src: R,
    ttl: isize,
    buf: Vec<u8>,
    stack: Vec<PunctStart>,
}

impl<'de, R: Source<'de>> Deserializer<R> {
    pub fn new(src: R) -> Self {
        Self {
            src,
            ttl: DEFAULT_RECURSION_LIMIT,
            buf: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn parse<T>(&mut self) -> Result<T> {
        if self.ttl < 0 {
            return Err(todo!("previous errored"));
        }

        todo!()
    }
}
