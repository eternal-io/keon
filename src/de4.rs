use self::{error::*, private::*};
use crate::{format::*, value::*};
use core::{cmp::Ordering, marker::PhantomData};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};

mod de_to_concr;
mod de_to_value;
pub mod error;
pub mod source;

pub struct Deserializer<R> {
    src: R,
    buf: Vec<u8>,
}

#[doc(hidden)]
pub trait DeserializerImpl<'de> {
    fn pull(&'de mut self) -> Result<Token<'de>>;

    fn finish(&mut self) -> Result;
}

mod private {
    pub enum Token<'de> {
        Str(&'de str),
    }
}
