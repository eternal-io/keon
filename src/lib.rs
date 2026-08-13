#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod de;
pub mod ser;
pub mod value;

pub use crate::ser::Serializer;

#[cfg(feature = "alloc")]
pub use crate::{
    ser::{stringify, stringify_pretty},
    value::Value2,
};

struct PrivateMethod;
