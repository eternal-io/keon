use self::error::*;
use core::fmt::{self, Write};

pub mod error;
pub mod ser_concr;
pub mod ser_value;

#[doc(alias = "compact_seria")]
pub fn seria<T: Seriable>(value: T) -> String {
    todo!()
}

#[doc(alias = "compact_seria_many")]
pub fn seria_many<T, I>(values: I)
where
    T: Seriable,
    I: Iterator<Item = T>,
{
    todo!()
}

pub fn pretty_seria<T: Seriable>(value: T) -> String {
    todo!()
}

pub fn custom_seria<T: Seriable>(value: T, style: Style) -> String {
    todo!()
}

//------------------------------------------------------------------------------

#[non_exhaustive]
pub struct Style {}

//------------------------------------------------------------------------------

#[doc(alias = "Serialize")]
pub trait Seriable {
    fn seria_via<W: Write>(&self, ser: &mut Serria<W>) -> SeriaResult;
}

//------------------------------------------------------------------------------

#[doc(alias = "Serializer")]
pub struct Serria<W: Write> {
    dst: W,
    style: Style,
}

impl<W: Write> Serria<W> {}
