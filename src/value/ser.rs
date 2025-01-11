use super::*;
use serde::{Serialize, Serializer};
use std::io::Write;

impl Value {
    pub fn to_string(&self) -> Result<String> {
        to_string(self)
    }
    pub fn to_string_pretty(&self) -> Result<String> {
        to_string_pretty(self)
    }
    pub fn to_writer<W: Write>(&self, writer: W) -> Result<()> {
        to_writer(writer, self)
    }
    pub fn to_writer_pretty<W: Write>(&self, writer: W) -> Result<()> {
        to_writer_pretty(writer, self)
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, ser: S) -> core::result::Result<S::Ok, S::Error> {
        match self {
            Value::Unit => ser.serialize_unit(),
            Value::Bool(b) => ser.serialize_bool(*b),
            Value::Char(ch) => ser.serialize_char(*ch),
            Value::Number(num) => match num {
                Number::Int(i) => ser.serialize_i64(*i),
                Number::UInt(u) => ser.serialize_u64(*u),
                Number::Float(f) => ser.serialize_f64(*f),
            },
            Value::String(s) => ser.serialize_str(s),
            Value::Bytes(bytes) => ser.serialize_bytes(bytes),
            Value::Newtype(obj) => ser.serialize_newtype_struct("", obj),
            Value::Opt(opt) => match opt {
                None => ser.serialize_none(),
                Some(v) => ser.serialize_some(v),
            },
            Value::Seq(seq) => ser.collect_seq(seq),
            Value::Map(map) => ser.collect_map(map),
        }
    }
}
