use std::{
    cmp::Ordering,
    collections::BTreeMap,
    hash::{Hash, Hasher},
    mem,
};

pub mod concr_to_value;
pub mod value_to_concr;

pub type Str = Box<str>;
pub type ByteBuf = Vec<u8>;
pub type Values = Vec<Value>;
pub type ValueMap = BTreeMap<Value, Value>;
pub type Struct = BTreeMap<Str, Value>;

#[derive(Debug, Clone, Copy)]
pub struct Float(pub f64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// Literal Boolean value.
    Bool(bool),

    /// Literal Unicode character.
    Char(char),

    /// Literal 64-bit floating-point number.
    Float(Float),

    /// Literal 64-bit unsigned integer.
    UInt64(u64),
    /// Literal 64-bit signed integer.
    /// Guaranteed to be a non-positive number if this value is parsed by KEON.
    SInt64(i64),

    /// Literal 128-bit unsigned integer.
    UInt128(Box<u128>),
    /// Literal 128-bit signed integer.
    /// Guaranteed to be a non-positive number if this value is parsed by KEON.
    SInt128(Box<i128>),

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
    /// Guaranteed that the [`Values`] inside is non-empty, if it's serialized/deserialized by KEON.
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
    Unnamed { stru: NominalValue },
    StemOnly { stru: NominalValue, name: Str },
    FullNamed { stru: NominalValue, name: Str, parent: Str },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NominalValue {
    Unit,
    Tuple(Values),
    Struct(Struct),
}

//------------------------------------------------------------------------------

macro_rules! impl_into {
    ( $value:ident: $from:ty => $t0:tt $($tt:tt)* ) => {
        impl From<$from> for $t0 {
            #[inline]
            #[allow(unused_variables)]
            fn from($value: $from) -> Self {
                $t0 $($tt)*
            }
        }
    };
}

impl_into!(v: bool    => Value::Bool(v));
impl_into!(v: char    => Value::Char(v));
impl_into!(v: f32     => Value::Float(v.into()));
impl_into!(v: f64     => Value::Float(v.into()));
impl_into!(v: u8      => Value::UInt64(v as _));
impl_into!(v: u16     => Value::UInt64(v as _));
impl_into!(v: u32     => Value::UInt64(v as _));
impl_into!(v: u64     => Value::UInt64(v as _));
impl_into!(v: u128    => Value::UInt128(Box::new(v)));
impl_into!(v: i8      => Value::SInt64(v as _));
impl_into!(v: i16     => Value::SInt64(v as _));
impl_into!(v: i32     => Value::SInt64(v as _));
impl_into!(v: i64     => Value::SInt64(v as _));
impl_into!(v: i128    => Value::SInt128(Box::new(v)));
impl_into!(v: String  => Value::String(Box::new(v)));
impl_into!(v: &str    => Value::String(Box::new(v.into())));
impl_into!(v: ByteBuf => Value::ByteBuf(Box::new(v)));
impl_into!(v: &[u8]   => Value::ByteBuf(Box::new(v.into())));
impl_into!(v: ()      => Value::Tuple(None));

//------------------------------------------------------------------------------

impl_into!(v: f32 => Float(v as _));
impl_into!(v: f64 => Float(v as _));

impl Eq for Float {}

impl PartialEq for Float {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let (x, y) = (self.0, other.0);
        x.is_nan() && y.is_nan() || x == y
    }
}

impl Ord for Float {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for Float {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Float {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.to_bits());
    }
}

//------------------------------------------------------------------------------

impl Nominal {
    #[inline]
    pub fn set_struct(&mut self, stru: NominalValue) -> NominalValue {
        match self {
            Nominal::Unnamed { stru: stru_ }
            | Nominal::StemOnly { stru: stru_, .. }
            | Nominal::FullNamed { stru: stru_, .. } => mem::replace(stru_, stru),
        }
    }
}
