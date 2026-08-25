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

#[doc(hidden)]
#[cfg(feature = "alloc")]
pub fn _test_roundtrip<T>(object: &T) -> Result
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
{
    let repr_p0 = stringify(object).unwrap();
    let repr_p1 = stringify_pretty(object).unwrap();

    let value = from_str(&repr_p0)?;
    assert_eq!(value, from_str(&repr_p1)?);
    assert_eq!(value, from_bytes(repr_p0.as_bytes())?);
    assert_eq!(value, from_bytes(repr_p1.as_bytes())?);

    let back_p0 = parse_str::<T>(&repr_p0)?;
    let back_p1 = parse_str::<T>(&repr_p1)?;
    assert_eq!(object, &back_p0);
    assert_eq!(object, &back_p1);

    let back_pv = value.deserialize_to::<T>()?;
    assert_eq!(object, &back_pv);

    Ok(())
}
