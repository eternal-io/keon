use std::{
    cmp::Ordering,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

pub mod de_to_concr;
pub mod ser_to_value;

pub type Str = Box<str>;
pub type ByteBuf = Vec<u8>;
pub type Values = Vec<Value>;
pub type ValueMap = BTreeMap<Value, Value>;
pub type Record = BTreeMap<Str, Value>;

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Nat(u64),
    Int(i64),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// Literal bool.
    Bool(bool),

    /// Literal char.
    Char(char),

    /// Literal number.
    Number(Number),

    /// Literal string.
    String(Box<String>),

    /// Literal byte string.
    ByteBuf(Box<ByteBuf>),

    /// Structural maybe, either `Some(Value)` or `None`.
    ///
    /// This is non-nominal due to [serde]'s design.
    Maybe(Option<Box<Value>>),

    /// Structural tuple, also used to represent 'unit'.
    ///
    /// Guaranteed that the [`Values`] inside is non-empty, if it's provided by KEON.
    Tuple(Option<Box<Values>>),

    /// Structural seq.
    Seq(Box<Values>),

    /// Structural map.
    Map(Box<ValueMap>),

    /// Nominal value.
    Nominal(Box<Nominal>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Nominal {
    Unnamed { stru: Struct },
    StemOnly { stru: Struct, stem: Str },
    FullNamed { stru: Struct, stem: Str, parent: Str },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Struct {
    /// Aka 'unit'.
    Tuple0,
    /// Aka 'newtype', specialized due to [serde]'s design.
    Tuple1(Value),
    /// Just 'tuple', guaranteed that it has at least two values, if it's provided by KEON.
    TupleN(Values),
    /// Aka 'struct'.
    Record(Record),
}

//------------------------------------------------------------------------------

macro_rules! impl_into {
    ( $value:ident: $from:ty => $into:ident::$($tt:tt)* ) => {
        impl From<$from> for $into {
            #[inline]
            #[allow(unused_variables)]
            fn from($value: $from) -> Self {
                $into::$($tt)*
            }
        }
    };
}

impl_into!( v: bool  => Value::Bool(v) );
impl_into!( v: char  => Value::Char(v) );
impl_into!( v: i8    => Value::Number(v.into()) );
impl_into!( v: i16   => Value::Number(v.into()) );
impl_into!( v: i32   => Value::Number(v.into()) );
impl_into!( v: i64   => Value::Number(v.into()) );
impl_into!( v: u8    => Value::Number(v.into()) );
impl_into!( v: u16   => Value::Number(v.into()) );
impl_into!( v: u32   => Value::Number(v.into()) );
impl_into!( v: u64   => Value::Number(v.into()) );
impl_into!( v: f32   => Value::Number(v.into()) );
impl_into!( v: f64   => Value::Number(v.into()) );
impl_into!( v: &str  => Value::String(Box::new(v.into())) );
impl_into!( v: &[u8] => Value::ByteBuf(Box::new(v.into())) );
impl_into!( v: ()    => Value::Tuple(None) );

impl_into!( v: u8    => Number::Nat(v as _) );
impl_into!( v: u16   => Number::Nat(v as _) );
impl_into!( v: u32   => Number::Nat(v as _) );
impl_into!( v: u64   => Number::Nat(v as _) );
impl_into!( v: usize => Number::Nat(v as _) );
impl_into!( v: i8    => Number::Int(v as _) );
impl_into!( v: i16   => Number::Int(v as _) );
impl_into!( v: i32   => Number::Int(v as _) );
impl_into!( v: i64   => Number::Int(v as _) );
impl_into!( v: isize => Number::Int(v as _) );
impl_into!( v: f32   => Number::Float(v as _) );
impl_into!( v: f64   => Number::Float(v as _) );

//------------------------------------------------------------------------------

// impl From<Option<Box<Value>>> for Container {
//     fn from(value: Option<Box<Value>>) -> Self {
//         Self::Maybe(value)
//     }
// }

// impl<T: Into<Value>> From<Option<T>> for Container {
//     fn from(value: Option<T>) -> Self {
//         Self::Maybe(value.map(|v| Box::new(v.into())))
//     }
// }

// impl<T> From<&[T]> for Container
// where
//     T: Into<Value> + Clone,
// {
//     fn from(value: &[T]) -> Self {
//         Self::Seq(Box::new(Vec::from_iter(value.iter().cloned().map(Into::into))))
//     }
// }

// impl<T, const N: usize> From<[T; N]> for Container
// where
//     T: Into<Value>,
// {
//     fn from(value: [T; N]) -> Self {
//         Self::Seq(Box::new(Vec::from_iter(value.into_iter().map(Into::into))))
//     }
// }

// impl<K, V> From<&[(K, V)]> for Container
// where
//     K: Into<Value> + Clone,
//     V: Into<Value> + Clone,
// {
//     fn from(value: &[(K, V)]) -> Self {
//         Self::Map(Box::new(BTreeMap::from_iter(
//             value.iter().cloned().map(|(k, v)| (k.into(), v.into())),
//         )))
//     }
// }

// impl<K, V, const N: usize> From<[(K, V); N]> for Container
// where
//     K: Into<Value>,
//     V: Into<Value>,
// {
//     fn from(value: [(K, V); N]) -> Self {
//         Self::Map(Box::new(BTreeMap::from_iter(
//             value.into_iter().map(|(k, v)| (k.into(), v.into())),
//         )))
//     }
// }

//------------------------------------------------------------------------------

impl Number {
    #[inline]
    pub const fn to_i64_lossy(self) -> i64 {
        match self {
            Self::Nat(n) => match n >= i64::MAX as u64 {
                true => i64::MAX,
                false => n as i64,
            },
            Self::Int(i) => i,
            Self::Float(f) => f.clamp(i64::MIN as f64, i64::MAX as f64) as i64,
        }
    }

    #[inline]
    pub const fn to_u64_lossy(self) -> u64 {
        match self {
            Self::Nat(n) => n,
            Self::Int(i) => match i >= 0 {
                true => i as u64,
                false => 0,
            },
            Self::Float(f) => f.clamp(u64::MIN as f64, u64::MAX as f64) as u64,
        }
    }

    #[inline]
    pub const fn to_f64_lossy(self) -> f64 {
        match self {
            Self::Nat(n) => n as f64,
            Self::Int(i) => i as f64,
            Self::Float(f) => f,
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nat(a), Self::Nat(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.is_nan() && b.is_nan() || a == b,
            _ => false,
        }
    }
}

impl Eq for Number {}

/// In order to be able to use [`Number`] as a map key,
/// `NaN` is greater than any other number and equal to themselves.
#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match self {
            Number::Nat(n) => match other {
                Number::Nat(m) => n.cmp(m),
                Number::Int(_) => Ordering::Greater,
                Number::Float(_) => Ordering::Less,
            },
            Number::Int(i) => match other {
                Number::Nat(_) => Ordering::Less,
                Number::Int(j) => i.cmp(j),
                Number::Float(_) => Ordering::Less,
            },
            Number::Float(f) => match other {
                Number::Nat(_) => Ordering::Greater,
                Number::Int(_) => Ordering::Greater,
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
            Number::Nat(n) => state.write_u64(*n),
            Number::Int(i) => state.write_i64(*i),
            Number::Float(f) => state.write_u64(f.to_bits()),
        }
    }
}
