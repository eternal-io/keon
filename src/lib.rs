pub mod de;
pub mod error;
pub mod ser;
pub mod value;

#[doc(inline)]
pub use crate::{
    de::{parse, parse_limited, parse_many, parse_many_limited},
    error::{Error, ErrorKind},
    value::Value,
};

pub(crate) use crate::error::Result;
pub(crate) use core::result::Result as StdResult;
