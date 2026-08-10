#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod de4;
pub mod ser4;
pub mod value;

pub use crate::ser4::Serializer;

#[cfg(feature = "alloc")]
pub use crate::{
    ser4::{stringify, stringify_pretty},
    value::Value2,
};

struct PrivateMethod;
