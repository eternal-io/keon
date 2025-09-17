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
    Unsigned64(u64),
    /// Literal 64-bit signed integer.
    /// Guaranteed to be a negative number if this value is parsed by KEON.
    Negative64(i64),

    /// Literal 128-bit unsigned integer.
    Unsigned128(Box<u128>),
    /// Literal 128-bit signed integer.
    /// Guaranteed to be a negative number if this value is parsed by KEON.
    Negative128(Box<i128>),

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
    Unnamed { stru: Struct },
    StemOnly { stru: Struct, name: Str },
    FullNamed { stru: Struct, name: Str, parent: Str },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Struct {
    Unit,
    Tuple(Values),
    Record(Record),
}

//------------------------------------------------------------------------------

macro_rules! impl_into {
    ( $out:ty | $value:ident: $from:ty => $($tt:tt)* ) => {
        impl From<$from> for $out {
            #[inline]
            #[allow(unused_variables)]
            fn from($value: $from) -> Self {
                $($tt)*
            }
        }
    };
}

impl_into!( Value | v: bool    => Value::Bool(v) );
impl_into!( Value | v: char    => Value::Char(v) );
impl_into!( Value | v: f32     => Value::Float(v.into()) );
impl_into!( Value | v: f64     => Value::Float(v.into()) );
impl_into!( Value | v: u8      => Value::Unsigned64(v as _) );
impl_into!( Value | v: u16     => Value::Unsigned64(v as _) );
impl_into!( Value | v: u32     => Value::Unsigned64(v as _) );
impl_into!( Value | v: u64     => Value::Unsigned64(v as _) );
impl_into!( Value | v: u128    => Value::Unsigned128(Box::new(v)) );
impl_into!( Value | v: i8      => Value::Negative64(v as _) );
impl_into!( Value | v: i16     => Value::Negative64(v as _) );
impl_into!( Value | v: i32     => Value::Negative64(v as _) );
impl_into!( Value | v: i64     => Value::Negative64(v as _) );
impl_into!( Value | v: i128    => Value::Negative128(Box::new(v)) );
impl_into!( Value | v: String  => Value::String(Box::new(v)) );
impl_into!( Value | v: &str    => Value::String(Box::new(v.into())) );
impl_into!( Value | v: ByteBuf => Value::ByteBuf(Box::new(v)) );
impl_into!( Value | v: &[u8]   => Value::ByteBuf(Box::new(v.into())) );
impl_into!( Value | v: ()      => Value::Tuple(None) );

//------------------------------------------------------------------------------

impl_into!( Float | v: f32 => Float(v as _) );
impl_into!( Float | v: f64 => Float(v as _) );

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
