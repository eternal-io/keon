use alloc::collections::BTreeMap;
use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    mem,
    ops::Deref,
};

pub mod concr_to_value;
pub mod value_to_concr;

pub type Str = Box<str>;
pub type ByteBuf = Vec<u8>;
pub type Values = Vec<Value>;
pub type ValueMap = BTreeMap<Value, Value>;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Float32(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Float64(pub f64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Number {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(Box<i128>),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    UInt128(Box<u128>),
    Float32(Float32),
    Float64(Float64),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumberNoSuffix {
    Int(i64),
    UInt(u64),
    Float(Float64),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// Literal Boolean value.
    Bool(bool),

    /// Literal Unicode character.
    Char(char),

    /// Literal number.
    Number(Number),

    /// Literal number with no suffix.
    NumberNoSuffix(NumberNoSuffix),

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
    /// Guaranteed that the [`Values`] inside is non-empty, if it's (de)serialized by KEON.
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
    Struct(BTreeMap<Str, Value>),
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
impl_into!(v: f32     => Value::Number(v.into()));
impl_into!(v: f64     => Value::Number(v.into()));
impl_into!(v: i8      => Value::Number(v.into()));
impl_into!(v: i16     => Value::Number(v.into()));
impl_into!(v: i32     => Value::Number(v.into()));
impl_into!(v: i64     => Value::Number(v.into()));
impl_into!(v: i128    => Value::Number(v.into()));
impl_into!(v: u8      => Value::Number(v.into()));
impl_into!(v: u16     => Value::Number(v.into()));
impl_into!(v: u32     => Value::Number(v.into()));
impl_into!(v: u64     => Value::Number(v.into()));
impl_into!(v: u128    => Value::Number(v.into()));
impl_into!(v: String  => Value::String(Box::new(v)));
impl_into!(v: &str    => Value::String(Box::new(v.into())));
impl_into!(v: ByteBuf => Value::ByteBuf(Box::new(v)));
impl_into!(v: &[u8]   => Value::ByteBuf(Box::new(v.into())));
impl_into!(v: ()      => Value::Tuple(None));

impl_into!(v: f32  => Number::Float32(v.into()));
impl_into!(v: f64  => Number::Float64(v.into()));
impl_into!(v: i8   => Number::Int8(v));
impl_into!(v: i16  => Number::Int16(v));
impl_into!(v: i32  => Number::Int32(v));
impl_into!(v: i64  => Number::Int64(v));
impl_into!(v: i128 => Number::Int128(Box::new(v)));
impl_into!(v: u8   => Number::UInt8(v));
impl_into!(v: u16  => Number::UInt16(v));
impl_into!(v: u32  => Number::UInt32(v));
impl_into!(v: u64  => Number::UInt64(v));
impl_into!(v: u128 => Number::UInt128(Box::new(v)));

impl_into!(v: i64  => NumberNoSuffix::Int(v));
impl_into!(v: u64  => NumberNoSuffix::UInt(v));
impl_into!(v: f64  => NumberNoSuffix::Float(v.into()));

//------------------------------------------------------------------------------

impl_into!(v: Number2 => Value2::Number(v));

impl_into!(v: i8   => Number2::Int8(v));
impl_into!(v: i16  => Number2::Int16(v));
impl_into!(v: i32  => Number2::Int32(v));
impl_into!(v: i64  => Number2::Int64(v));
impl_into!(v: i128 => Number2::Int128 { lo: v as _, hi: (v >> 64) as _ });
impl_into!(v: u8   => Number2::UInt8(v));
impl_into!(v: u16  => Number2::UInt16(v));
impl_into!(v: u32  => Number2::UInt32(v));
impl_into!(v: u64  => Number2::UInt64(v));
impl_into!(v: u128 => Number2::UInt128 { lo: v as _, hi: (v >> 64) as _ });
impl_into!(v: f32  => Number2::Float32(v.into()));
impl_into!(v: f64  => Number2::Float64(v.into()));

impl_into!(v: f32  => Float32(v));
impl_into!(v: f32  => Float64(v as _));
impl_into!(v: f64  => Float64(v));

pub type Values2 = Vec<Value2>;
pub type ValuesMap2 = BTreeMap<Value2, Value2>;
pub type FieldsMap2 = BTreeMap<IdentBuf, Value2>;
pub type IdentBuf = Box<Ident>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Number2 {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128 { lo: u64, hi: i64 },
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    UInt128 { lo: u64, hi: u64 },
    Float32(Float32),
    Float64(Float64),

    IntNoSuffix(i64),
    UIntNoSuffix(u64),
    FloatNoSuffix(Float64),
}

pub(crate) enum NumberSuffix {
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    Float32,
    Float64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scalar {
    Char(char),
    Number(Number2),
}

impl Into<Value2> for Scalar {
    fn into(self) -> Value2 {
        match self {
            Scalar::Char(ch) => Value2::Char(ch),
            Scalar::Number(num) => Value2::Number(num),
        }
    }
}

impl<'a> TryFrom<&'a Value2> for Scalar {
    type Error = &'a Value2;

    fn try_from(value: &'a Value2) -> Result<Self, Self::Error> {
        match value {
            Value2::Char(ch) => Ok(Scalar::Char(*ch)),
            Value2::Number(num) => Ok(Scalar::Number(*num)),
            non_scalar => Err(non_scalar),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident(str);

impl Ident {
    pub fn new(ident: &str) -> Option<&Self> {
        let mut chars = ident.chars();
        let mut accept = true;
        match chars.next()? {
            '_' => accept = false,
            ch => unicode_ident::is_xid_start(ch).then_some(())?,
        }
        for ch in chars {
            unicode_ident::is_xid_continue(ch).then_some(())?;
            accept = true;
        }

        accept.then_some(Self::new_unchecked(ident))
    }

    #[inline]
    pub const fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub(crate) const fn new_unchecked(ident: &str) -> &Self {
        unsafe { core::mem::transmute::<&str, &Self>(ident) }
    }
}

impl Deref for Ident {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ToOwned for Ident {
    type Owned = Box<Self>;
    fn to_owned(&self) -> Self::Owned {
        unsafe { core::mem::transmute::<Box<str>, Box<Self>>(Box::from(&self.0)) }
    }
}

impl Clone for Box<Ident> {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value2 {
    /// Literal Boolean value.
    Bool(bool),
    /// Literal Unicode character.
    Char(char),
    /// Literal number.
    Number(Number2),
    /// Literal string.
    String(Box<str>),
    /// Literal byte string.
    ByteBuf(Box<[u8]>),
    /// Structural unit.
    Unit,

    UnitStruct(Option<Box<IdentBuf>>),

    UnitVariant(Box<Variant>),

    /// `..`
    RangeFull,
    /// `..q`
    RangeTo(Box<Scalar>),
    /// `..=q`
    RangeToInclusive(Box<Scalar>),
    /// `p..`
    RangeFrom(Box<Scalar>),
    /// `p..q`
    Range(Box<(Scalar, Scalar)>),
    /// `p..=q`
    RangeInclusive(Box<(Scalar, Scalar)>),

    /// Maybe value, either [`Some`] or [`None`].
    ///
    /// This is non-nominal due to [serde]'s design.
    Maybe(Option<Box<Value2>>),

    /// Structural array.
    Array(Box<Values2>),

    /// Structural tuple.
    Tuple(Box<Values2>),

    TupleStruct(Box<Struct<Values2>>),

    TupleVariant(Box<Variant<Values2>>),

    /// Structural map.
    Map(Box<ValuesMap2>),

    MapStruct(Box<Struct<FieldsMap2>>),

    MapVariant(Box<Variant<FieldsMap2>>),

    /// Newtype struct.
    Newtype(Box<Struct<Value2>>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Struct<T> {
    pub name: Option<IdentBuf>,
    pub body: T,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant<T = ()> {
    pub name: Option<IdentBuf>,
    pub variant: IdentBuf,
    pub body: T,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NominalPath2 {
    Underscore,
    Single { name: IdentBuf },
    Dual { name: IdentBuf, parent: IdentBuf },
}

impl NominalPath2 {
    // TODO: convenient methods
}

pub(crate) enum NominalPathRef<'a> {
    Underscore,
    Single { name: &'a Ident },
    Dual { name: &'a Ident, parent: &'a Ident },
}

impl From<NominalPathRef<'_>> for NominalPath2 {
    fn from(value: NominalPathRef<'_>) -> Self {
        match value {
            NominalPathRef::Underscore => NominalPath2::Underscore,
            NominalPathRef::Single { name } => NominalPath2::Single { name: name.to_owned() },
            NominalPathRef::Dual { name, parent } => NominalPath2::Dual {
                name: name.to_owned(),
                parent: parent.to_owned(),
            },
        }
    }
}

impl<'a> From<&'a NominalPath2> for NominalPathRef<'a> {
    #[inline(always)]
    fn from(value: &'a NominalPath2) -> Self {
        match value {
            NominalPath2::Underscore => NominalPathRef::Underscore,
            NominalPath2::Single { name } => NominalPathRef::Single { name: name.borrow() },
            NominalPath2::Dual { name, parent } => NominalPathRef::Dual {
                name: name.borrow(),
                parent: parent.borrow(),
            },
        }
    }
}

//------------------------------------------------------------------------------

impl Eq for Float32 {}

impl PartialEq for Float32 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let (x, y) = (self.0, other.0);
        x.is_nan() && y.is_nan() || x == y
    }
}

impl Ord for Float32 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for Float32 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Float32 {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.0.to_bits());
    }
}

impl Eq for Float64 {}

impl PartialEq for Float64 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let (x, y) = (self.0, other.0);
        x.is_nan() && y.is_nan() || x == y
    }
}

impl Ord for Float64 {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for Float64 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Float64 {
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

#[test]
fn foo() {
    let _a = 1_1.;
    let _num = "1_1.".parse::<f64>().unwrap();
}
