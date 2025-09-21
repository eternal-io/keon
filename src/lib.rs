pub mod de;
pub mod ser;
pub mod value;

#[doc(inline)]
pub use crate::{
    de::{
        error::{Error, ErrorKind},
        parse, parse_limited, parse_many, parse_many_limited,
    },
    value::Value,
};
