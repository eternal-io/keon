#[cfg(feature = "alloc")]
use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, vec::Vec};
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
};

#[cfg(feature = "alloc")]
pub mod concr_to_value;
pub mod scalar_to_concr;
#[cfg(feature = "alloc")]
pub mod value_to_concr;

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

impl_into!(v: char    => Scalar::Char(v));
impl_into!(v: Number2 => Scalar::Number(v));
impl_into!(v:&Number2 => Scalar::Number(*v));
impl_into!(v: i8      => Scalar::Number(v.into()));
impl_into!(v: i16     => Scalar::Number(v.into()));
impl_into!(v: i32     => Scalar::Number(v.into()));
impl_into!(v: i64     => Scalar::Number(v.into()));
impl_into!(v: i128    => Scalar::Number(v.into()));
impl_into!(v: u8      => Scalar::Number(v.into()));
impl_into!(v: u16     => Scalar::Number(v.into()));
impl_into!(v: u32     => Scalar::Number(v.into()));
impl_into!(v: u64     => Scalar::Number(v.into()));
impl_into!(v: u128    => Scalar::Number(v.into()));
impl_into!(v: f32     => Scalar::Number(v.into()));
impl_into!(v: f64     => Scalar::Number(v.into()));

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

impl Number2 {
    #[inline]
    pub(crate) fn suffix(&self) -> Option<NumberSuffix> {
        'suff: {
            let suff = match self {
                Number2::Int8(_) => NumberSuffix::Int8,
                Number2::Int16(_) => NumberSuffix::Int16,
                Number2::Int32(_) => NumberSuffix::Int32,
                Number2::Int64(_) => NumberSuffix::Int64,
                Number2::Int128 { .. } => NumberSuffix::Int128,
                Number2::UInt8(_) => NumberSuffix::UInt8,
                Number2::UInt16(_) => NumberSuffix::UInt16,
                Number2::UInt32(_) => NumberSuffix::UInt32,
                Number2::UInt64(_) => NumberSuffix::UInt64,
                Number2::UInt128 { .. } => NumberSuffix::UInt128,
                Number2::Float32(_) => NumberSuffix::Float32,
                Number2::Float64(_) => NumberSuffix::Float64,
                _ => break 'suff,
            };
            return Some(suff);
        }
        None
    }
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

#[cfg(feature = "alloc")]
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

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Struct<T> {
    pub name: Option<IdentBuf>,
    pub body: T,
}

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant<T = ()> {
    pub name: Option<IdentBuf>,
    pub variant: IdentBuf,
    pub body: T,
}

//------------------------------------------------------------------------------

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Float32(pub f32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Float64(pub f64);

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
