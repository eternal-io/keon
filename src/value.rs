use super::*;
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

/// Implementing [`Deserialize`] for Value.
mod de;
/// Implementing [`Serialize`] for Value.
mod ser;

pub type ByteBuf = Vec<u8>;
pub type Seq = Vec<Value>;
pub type Map = BTreeMap<Value, Value>;

/// Due to the limitation of the [serde], enum variants cannot roundtrip via [`Value`].
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    #[default]
    Unit,
    Bool(bool),
    Char(char),
    Number(Number),
    String(String),
    Bytes(ByteBuf),
    Newtype(Box<Value>),
    Opt(Option<Box<Value>>),
    Seq(Seq),
    Map(Map),
}

/// A wrapper for a number, can be one of `i64`, `u64` or `f64`.
///
/// In the deserialized outputs, the `i64` in `Int` is always negative.
#[derive(Debug, Clone, Copy)]
pub enum Number {
    Int(i64),
    UInt(u64),
    Float(f64),
}

//------------------------------------------------------------------------------
macro_rules! impl_into {
    ($ty:ty { $($v:ident @ $from_ty:ty => $expr:expr,)* }) => {
        $(
            impl From<$from_ty> for $ty {
                #[allow(unused_variables)]
                fn from($v: $from_ty) -> Self {
                    $expr
                }
            }
        )*
    };
}

impl_into! {
    Value {
        v @ () => Value::Unit,
        v @ bool => Value::Bool(v),
        v @ char => Value::Char(v),

        v @ i8 => Value::from(v as i64),
        v @ i16 => Value::from(v as i64),
        v @ i32 => Value::from(v as i64),
        v @ i64 => Value::Number(Number::Int(v)),

        v @ u8 => Value::from(v as u64),
        v @ u16 => Value::from(v as u64),
        v @ u32 => Value::from(v as u64),
        v @ u64 => Value::Number(Number::UInt(v)),

        v @ f32 => Value::from(v as f64),
        v @ f64 => Value::Number(Number::Float(v)),

        v @ &str => Value::String(v.to_string()),
        v @ String => Value::String(v),

        v @ &[u8] => Value::Bytes(v.to_vec()),
        v @ ByteBuf => Value::Bytes(v),

        v @ Box<Value> => Value::Newtype(v),
        v @ Option<Value> => Value::Opt(v.map(Box::new)),
        v @ Seq => Value::Seq(v),
        v @ Map => Value::Map(v),
    }
}

//------------------------------------------------------------------------------
impl Number {
    pub fn saturating_into_i64(self) -> i64 {
        match self {
            Self::Int(i) => i,
            Self::UInt(u) => match u >= i64::MAX as u64 {
                true => i64::MAX,
                false => u as i64,
            },
            Self::Float(f) => f.clamp(i64::MIN as f64, i64::MAX as f64) as i64,
        }
    }

    pub fn saturating_into_u64(self) -> u64 {
        match self {
            Self::Int(i) => match i >= 0 {
                true => i as u64,
                false => 0,
            },
            Self::UInt(u) => u,
            Self::Float(f) => f.clamp(u64::MIN as f64, u64::MAX as f64) as u64,
        }
    }

    pub fn into_f64(self) -> f64 {
        match self {
            Self::Int(i) => i as f64,
            Self::UInt(u) => u as f64,
            Self::Float(f) => f,
        }
    }

    pub fn map<T>(
        self,
        int_fn: impl FnOnce(i64) -> T,
        uint_fn: impl FnOnce(u64) -> T,
        float_fn: impl FnOnce(f64) -> T,
    ) -> T {
        match self {
            Self::Int(i) => int_fn(i),
            Self::UInt(u) => uint_fn(u),
            Self::Float(f) => float_fn(f),
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::UInt(a), Self::UInt(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.is_nan() && b.is_nan() || a == b,
            _ => false,
        }
    }
}

impl Eq for Number {}

/// *`NaN` is greater then any other number, and equal to themselves.*
#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match self {
            Number::Int(i) => match other {
                Number::Int(j) => i.cmp(j),
                Number::UInt(_) => Ordering::Less,
                Number::Float(_) => Ordering::Less,
            },
            Number::UInt(u) => match other {
                Number::Int(_) => Ordering::Greater,
                Number::UInt(v) => u.cmp(v),
                Number::Float(_) => Ordering::Less,
            },
            Number::Float(f) => match other {
                Number::Int(_) => Ordering::Greater,
                Number::UInt(_) => Ordering::Greater,
                Number::Float(g) => match (f.is_nan(), g.is_nan()) {
                    (false, false) => f.partial_cmp(g).unwrap(),
                    (false, true) => Ordering::Less,
                    (true, false) => Ordering::Greater,
                    (true, true) => Ordering::Equal,
                },
            },
        })
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Number::Int(i) => state.write_i64(*i),
            Number::UInt(u) => state.write_u64(*u),
            Number::Float(f) => state.write_u64(f.to_bits()),
        }
    }
}
