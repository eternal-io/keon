pub mod de;
pub mod error;
pub mod ser;
pub mod value;

#[doc(inline)]
pub use crate::{
    de::{parse, parse_many},
    error::{Error, ErrorKind, Result},
    value::Value,
};
