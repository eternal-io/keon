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
        Deserializer,
    },
    ser::{stringify, stringify_pretty},
    value::Value,
};

struct PrivateMethod;
