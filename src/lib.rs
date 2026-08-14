#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod de;
pub mod ser;
pub mod value;

#[doc(inline)]
pub use crate::ser::Serializer;

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use crate::{
    de::{
        error::{Error, Result},
        from_bytes, from_str, parse_bytes, parse_str, Deserializer,
    },
    ser::{stringify, stringify_pretty},
    value::Value,
};

struct PrivateMethod;
