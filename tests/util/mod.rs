#![allow(unused)]

use core::fmt::Debug;
use serde::{de::DeserializeOwned, Serialize};

type AssertResult<T> = Result<T, Msg>;

pub struct Msg(String);
impl From<String> for Msg {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn rt_min<T: Debug + PartialEq + Serialize + DeserializeOwned>(obj: &T, expected: &str) -> AssertResult<()> {
    roundtrip(obj, &serialize_min(obj)?, expected)
}
pub fn rt_pre<T: Debug + PartialEq + Serialize + DeserializeOwned>(obj: &T, expected: &str) -> AssertResult<()> {
    roundtrip(obj, &serialize_pre(obj)?, expected)
}

pub fn forward<T: Debug + PartialEq + Serialize + DeserializeOwned>(obj: &T, expected: &str) -> AssertResult<()> {
    //! always "_min".
    let output = serialize_min(obj)?;
    (output == expected)
        .then_some(())
        .ok_or(format!("forward:\nLeft: {}\nRight: {}", output, expected).into())
}

pub fn backward<T: Debug + PartialEq + Serialize + DeserializeOwned>(obj: &T, s: &str) -> AssertResult<()> {
    let obj_back = deserialize::<T>(s)?;
    (*obj == obj_back)
        .then_some(())
        .ok_or(format!("backward:\nLeft: {:?}\nRight: {:?}", obj, obj_back).into())
}

//------------------------------------------------------------------------------

fn serialize_min<T: Debug + PartialEq + Serialize + DeserializeOwned>(obj: &T) -> AssertResult<String> {
    Ok(keon::to_string(obj).map_err(|e| format!("serialize: {}", e))?)
}
fn serialize_pre<T: Debug + PartialEq + Serialize + DeserializeOwned>(obj: &T) -> AssertResult<String> {
    Ok(keon::to_string_pretty(obj).map_err(|e| format!("serialize: {}", e))?)
}

fn deserialize<T: Debug + PartialEq + Serialize + DeserializeOwned>(s: &str) -> AssertResult<T> {
    Ok(keon::from_str(s).map_err(|e| format!("deserialize: {}", e))?)
}

fn roundtrip<T: Debug + PartialEq + Serialize + DeserializeOwned>(
    obj: &T,
    output: &str,
    expected: &str,
) -> AssertResult<()> {
    (output == expected)
        .then_some(())
        .ok_or(format!("forward:\nLeft: {}\nRight: {}", output, expected))?;

    let obj_back = deserialize::<T>(output)?;
    (*obj == obj_back)
        .then_some(())
        .ok_or(format!("backward:\nLeft: {:?}\nRight: {:?}", obj, obj_back))?;

    Ok(())
}
