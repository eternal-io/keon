use super::*;
use core::result::Result as StdResult;
use serde::{
    de::{DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};

impl Value {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self> {
        from_str(s)
    }

    /// Tries to deserialize this [`Value`] into `T`.
    pub fn into_rust<T: DeserializeOwned>(self) -> Result<T> {
        T::deserialize(self)
    }
}

impl core::str::FromStr for Value {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::from_str(s)
    }
}

//------------------------------------------------------------------------------

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(der: D) -> StdResult<Self, D::Error> {
        der.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;
impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn visit_unit<E: serde::de::Error>(self) -> StdResult<Self::Value, E> {
        Ok(Value::Unit)
    }

    fn visit_bool<E: serde::de::Error>(self, v: bool) -> StdResult<Self::Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> StdResult<Self::Value, E> {
        Ok(Value::Number(Number::Int(v)))
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> StdResult<Self::Value, E> {
        Ok(Value::Number(Number::UInt(v)))
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> StdResult<Self::Value, E> {
        Ok(Value::Number(Number::Float(v)))
    }

    fn visit_char<E: serde::de::Error>(self, v: char) -> StdResult<Self::Value, E> {
        Ok(Value::Char(v))
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> StdResult<Self::Value, E> {
        self.visit_string(v.to_string())
    }
    fn visit_borrowed_str<E: serde::de::Error>(self, v: &'de str) -> StdResult<Self::Value, E> {
        self.visit_string(v.to_string())
    }
    fn visit_string<E: serde::de::Error>(self, v: String) -> StdResult<Self::Value, E> {
        Ok(Value::String(v))
    }

    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> StdResult<Self::Value, E> {
        self.visit_byte_buf(v.to_vec())
    }
    fn visit_borrowed_bytes<E: serde::de::Error>(self, v: &'de [u8]) -> StdResult<Self::Value, E> {
        self.visit_byte_buf(v.to_vec())
    }
    fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> StdResult<Self::Value, E> {
        Ok(Value::Bytes(v))
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, deserializer: D) -> StdResult<Self::Value, D::Error> {
        Ok(Value::Newtype(Box::new(Value::deserialize(deserializer)?)))
    }

    fn visit_none<E: serde::de::Error>(self) -> StdResult<Self::Value, E> {
        Ok(Value::Opt(None))
    }
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> StdResult<Self::Value, D::Error> {
        Ok(Value::Opt(Some(Box::new(Value::deserialize(deserializer)?))))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq_accessor: A) -> StdResult<Self::Value, A::Error> {
        let mut seq = Seq::with_capacity(seq_accessor.size_hint().unwrap_or(128));
        while let Some(v) = seq_accessor.next_element()? {
            seq.push(v);
        }
        seq.shrink_to_fit();

        Ok(Value::Seq(seq))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map_accessor: A) -> StdResult<Self::Value, A::Error> {
        let mut map = Map::new();
        while let Some((k, v)) = map_accessor.next_entry()? {
            map.insert(k, v);
        }

        Ok(Value::Map(map))
    }

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("any value except i128, u128 or variant")
    }
}

//------------------------------------------------------------------------------

impl<'de> Deserializer<'de> for Value {
    type Error = Error;
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        match self {
            Value::Unit => vis.visit_unit(),
            Value::Bool(b) => vis.visit_bool(b),
            Value::Char(ch) => vis.visit_char(ch),
            Value::Number(number) => match number {
                Number::Int(i) => vis.visit_i64(i),
                Number::UInt(u) => vis.visit_u64(u),
                Number::Float(f) => vis.visit_f64(f),
            },
            Value::String(s) => vis.visit_string(s),
            Value::Bytes(buf) => vis.visit_byte_buf(buf),
            Value::Newtype(obj) => vis.visit_newtype_struct(*obj),
            Value::Opt(opt) => match opt {
                Some(v) => vis.visit_some(*v),
                None => vis.visit_none(),
            },
            Value::Seq(seq) => vis.visit_seq(SeqAccessor::new(seq)),
            Value::Map(map) => vis.visit_map(MapAccessor::new(map)),
        }
    }
}

struct SeqAccessor {
    seq: Seq,
    cursor: usize,
}
impl SeqAccessor {
    fn new(seq: Seq) -> Self {
        Self { seq, cursor: 0 }
    }
}
impl<'de> SeqAccess<'de> for SeqAccessor {
    type Error = Error;

    fn size_hint(&self) -> Option<usize> {
        Some(self.seq.len())
    }

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        Ok(match self.cursor == self.seq.len() {
            true => None,
            false => {
                let val = seed.deserialize(std::mem::take(&mut self.seq[self.cursor]))?;
                self.cursor += 1;
                Some(val)
            }
        })
    }
}

struct MapAccessor {
    map: Map,
    val: Option<Box<Value>>,
}
impl MapAccessor {
    fn new(map: Map) -> Self {
        Self { map, val: None }
    }
}
impl<'de> MapAccess<'de> for MapAccessor {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        Ok(match self.map.pop_first() {
            None => None,
            Some((k, v)) => Some({
                self.val = Some(Box::new(v));
                seed.deserialize(k)?
            }),
        })
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(*self.val.take().expect("contract violation"))
    }
}
