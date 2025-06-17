use super::*;
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

pub mod der_to_concr;
pub mod ser_to_value;

pub type Seq = Vec<Value>;
pub type Map = BTreeMap<Value, Value>;
pub type Struct = BTreeMap<EcoString, Value>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Literal(Literal),
    Structural(Structural),
    NamedStructural(EcoString, Structural),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Literal {
    Unit,
    Bool(bool),
    Char(char),
    Number(Number),
    String(EcoString),
    ByteBuf(EcoVec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Structural {
    Newtype(Box<Value>),
    Opt(Option<Box<Value>>),
    Seq(Seq),
    Map(Map),
    Struct(Struct),
}

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Int(i64),
    UInt(u64),
    Float(f64),
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

impl_into!( v: ()    => Literal::Unit );
impl_into!( v: bool  => Literal::Bool(v) );
impl_into!( v: char  => Literal::Char(v) );
impl_into!( v: i8    => Literal::Number(v.into()) );
impl_into!( v: i16   => Literal::Number(v.into()) );
impl_into!( v: i32   => Literal::Number(v.into()) );
impl_into!( v: i64   => Literal::Number(v.into()) );
impl_into!( v: u8    => Literal::Number(v.into()) );
impl_into!( v: u16   => Literal::Number(v.into()) );
impl_into!( v: u32   => Literal::Number(v.into()) );
impl_into!( v: u64   => Literal::Number(v.into()) );
impl_into!( v: f32   => Literal::Number(v.into()) );
impl_into!( v: f64   => Literal::Number(v.into()) );
impl_into!( v: &str  => Literal::String(v.into()) );
impl_into!( v: &[u8] => Literal::ByteBuf(v.into()) );

impl_into!( v: i8    => Number::Int(v as _) );
impl_into!( v: i16   => Number::Int(v as _) );
impl_into!( v: i32   => Number::Int(v as _) );
impl_into!( v: i64   => Number::Int(v as _) );
impl_into!( v: isize => Number::Int(v as _) );
impl_into!( v: u8    => Number::UInt(v as _) );
impl_into!( v: u16   => Number::UInt(v as _) );
impl_into!( v: u32   => Number::UInt(v as _) );
impl_into!( v: u64   => Number::UInt(v as _) );
impl_into!( v: usize => Number::UInt(v as _) );
impl_into!( v: f32   => Number::Float(v as _) );
impl_into!( v: f64   => Number::Float(v as _) );

// TODO: Into Value

//------------------------------------------------------------------------------

impl From<Box<Value>> for Structural {
    fn from(value: Box<Value>) -> Self {
        Self::Newtype(value)
    }
}

impl<T: Into<Value>> From<T> for Structural {
    fn from(value: T) -> Self {
        Self::Newtype(Box::new(value.into()))
    }
}

impl From<Option<Box<Value>>> for Structural {
    fn from(value: Option<Box<Value>>) -> Self {
        Self::Opt(value)
    }
}

impl<T: Into<Value>> From<Option<T>> for Structural {
    fn from(value: Option<T>) -> Self {
        Self::Opt(value.map(|v| Box::new(v.into())))
    }
}

impl<T> From<&[T]> for Structural
where
    T: Into<Value> + Clone,
{
    fn from(value: &[T]) -> Self {
        Self::Seq(Seq::from_iter(value.iter().cloned().map(Into::into)))
    }
}

impl<T, const N: usize> From<[T; N]> for Structural
where
    T: Into<Value>,
{
    fn from(value: [T; N]) -> Self {
        Self::Seq(Seq::from_iter(value.into_iter().map(Into::into)))
    }
}

impl<K, V> From<&[(K, V)]> for Structural
where
    K: Into<Value> + Clone,
    V: Into<Value> + Clone,
{
    fn from(value: &[(K, V)]) -> Self {
        Self::Map(Map::from_iter(value.iter().cloned().map(|(k, v)| (k.into(), v.into()))))
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for Structural
where
    K: Into<Value>,
    V: Into<Value>,
{
    fn from(value: [(K, V); N]) -> Self {
        Self::Map(Map::from_iter(value.into_iter().map(|(k, v)| (k.into(), v.into()))))
    }
}

//------------------------------------------------------------------------------

impl Number {
    #[inline]
    pub const fn saturating_to_i64(self) -> i64 {
        match self {
            Self::Int(i) => i,
            Self::UInt(u) => match u >= i64::MAX as u64 {
                true => i64::MAX,
                false => u as i64,
            },
            Self::Float(f) => f.clamp(i64::MIN as f64, i64::MAX as f64) as i64,
        }
    }

    #[inline]
    pub const fn saturating_to_u64(self) -> u64 {
        match self {
            Self::Int(i) => match i >= 0 {
                true => i as u64,
                false => 0,
            },
            Self::UInt(u) => u,
            Self::Float(f) => f.clamp(u64::MIN as f64, u64::MAX as f64) as u64,
        }
    }

    #[inline]
    pub const fn to_f64_lossy(self) -> f64 {
        match self {
            Self::Int(i) => i as f64,
            Self::UInt(u) => u as f64,
            Self::Float(f) => f,
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

/// In order to be able to use [`Number`] as a map key,
/// `NaN` is greater than any other number and equal to themselves.
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
