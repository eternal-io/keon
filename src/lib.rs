pub mod de;
pub mod error;
pub mod ser;
pub mod value;

#[doc(inline)]
pub use crate::{
    de::{parse, parse_many, parse_value, parse_values},
    error::{Error, ErrorKind, Result},
    value::Value,
};

pub(crate) use core::result::Result as StdResult;
