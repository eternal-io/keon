pub mod error;
pub mod parse;
pub mod value;

#[doc(inline)]
pub use crate::{
    error::{Error, ErrorKind, Result},
    parse::de_to_concr::parse,
    value::Value,
};
