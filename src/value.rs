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
#[cfg(feature = "alloc")]
pub mod scalar_to_concr;
#[cfg(feature = "alloc")]
pub mod value_to_concr;

macro_rules! impl_from_into {
    ($value:ident: $from:ty => $t0:tt $($tt:tt)*) => {
        impl From<$from> for $t0 {
            #[inline]
            #[allow(unused_variables)]
            fn from($value: $from) -> Self {
                $t0 $($tt)*
            }
        }
    };
}

//==================================================================================================

#[cfg(feature = "alloc")]
pub type Values = Vec<Value>;
#[cfg(feature = "alloc")]
pub type ValuesMap = BTreeMap<Value, Value>;
#[cfg(feature = "alloc")]
pub type FieldsMap = BTreeMap<IdentBuf, Value>;
#[cfg(feature = "alloc")]
pub type IdentBuf = Box<Ident>;

#[cfg(feature = "alloc")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    /// Literal Boolean value.
    Bool(bool),
    /// Literal Unicode character.
    Char(char),
    /// Literal number.
    Number(Number),
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
    Maybe(Option<Box<Value>>),

    /// Structural array.
    Array(Box<Values>),

    /// Structural tuple.
    Tuple(Box<Values>),

    TupleStruct(Box<Struct<Values>>),

    TupleVariant(Box<Variant<Values>>),

    /// Structural map.
    Map(Box<ValuesMap>),

    MapStruct(Box<Struct<FieldsMap>>),

    MapVariant(Box<Variant<FieldsMap>>),

    /// Newtype struct.
    Newtype(Box<Struct<Value>>),
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

#[cfg(feature = "alloc")]
impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Value::Number(value)
    }
}

#[cfg(feature = "alloc")]
impl From<Scalar> for Value {
    fn from(value: Scalar) -> Self {
        match value {
            Scalar::Char(ch) => Self::Char(ch),
            Scalar::Number(num) => Self::Number(num),
        }
    }
}

//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scalar {
    Char(char),
    Number(Number),
}

#[cfg(feature = "alloc")]
impl<'a> TryFrom<&'a Value> for Scalar {
    type Error = &'a Value;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Char(ch) => Ok(Scalar::Char(*ch)),
            Value::Number(num) => Ok(Scalar::Number(*num)),
            non_scalar => Err(non_scalar),
        }
    }
}

impl_from_into!(v: char    => Scalar::Char(v));
impl_from_into!(v: Number => Scalar::Number(v));
impl_from_into!(v:&Number => Scalar::Number(*v));
impl_from_into!(v: i8      => Scalar::Number(v.into()));
impl_from_into!(v: i16     => Scalar::Number(v.into()));
impl_from_into!(v: i32     => Scalar::Number(v.into()));
impl_from_into!(v: i64     => Scalar::Number(v.into()));
impl_from_into!(v: i128    => Scalar::Number(v.into()));
impl_from_into!(v: u8      => Scalar::Number(v.into()));
impl_from_into!(v: u16     => Scalar::Number(v.into()));
impl_from_into!(v: u32     => Scalar::Number(v.into()));
impl_from_into!(v: u64     => Scalar::Number(v.into()));
impl_from_into!(v: u128    => Scalar::Number(v.into()));
impl_from_into!(v: f32     => Scalar::Number(v.into()));
impl_from_into!(v: f64     => Scalar::Number(v.into()));

//==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Number {
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

impl Number {
    #[inline]
    pub(crate) fn suffix(&self) -> Option<NumberSuffix> {
        'suff: {
            Some(match self {
                Number::Int8(_) => NumberSuffix::Int8,
                Number::Int16(_) => NumberSuffix::Int16,
                Number::Int32(_) => NumberSuffix::Int32,
                Number::Int64(_) => NumberSuffix::Int64,
                Number::Int128 { .. } => NumberSuffix::Int128,
                Number::UInt8(_) => NumberSuffix::UInt8,
                Number::UInt16(_) => NumberSuffix::UInt16,
                Number::UInt32(_) => NumberSuffix::UInt32,
                Number::UInt64(_) => NumberSuffix::UInt64,
                Number::UInt128 { .. } => NumberSuffix::UInt128,
                Number::Float32(_) => NumberSuffix::Float32,
                Number::Float64(_) => NumberSuffix::Float64,
                _ => break 'suff None,
            })
        }
    }
}

impl_from_into!(v: i8   => Number::Int8(v));
impl_from_into!(v: i16  => Number::Int16(v));
impl_from_into!(v: i32  => Number::Int32(v));
impl_from_into!(v: i64  => Number::Int64(v));
impl_from_into!(v: i128 => Number::Int128 { lo: v as _, hi: (v >> 64) as _ });
impl_from_into!(v: u8   => Number::UInt8(v));
impl_from_into!(v: u16  => Number::UInt16(v));
impl_from_into!(v: u32  => Number::UInt32(v));
impl_from_into!(v: u64  => Number::UInt64(v));
impl_from_into!(v: u128 => Number::UInt128 { lo: v as _, hi: (v >> 64) as _ });
impl_from_into!(v: f32  => Number::Float32(v.into()));
impl_from_into!(v: f64  => Number::Float64(v.into()));

//==================================================================================================

impl_from_into!(v: f32  => Float32(v));
impl_from_into!(v: f64  => Float64(v));

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Float32(pub f32);

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

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Float64(pub f64);

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

//==================================================================================================

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
        // SAFETY: `Ident` is `#[repr(transparent)]` over `str`, so references
        // to `Ident` and `str` have identical representations. The conversion
        // preserves the original lifetime and underlying string data.
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

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

#[cfg(feature = "alloc")]
impl ToOwned for Ident {
    type Owned = Box<Self>;
    fn to_owned(&self) -> Self::Owned {
        // SAFETY: `Ident` is `#[repr(transparent)]` over `str`, making `Box<Ident>`
        // layout-compatible with `Box<str>`. The conversion preserves the allocation
        // and ownership of the boxed string.
        unsafe { core::mem::transmute::<Box<str>, Box<Self>>(Box::from(&self.0)) }
    }
}

#[cfg(feature = "alloc")]
impl Clone for Box<Ident> {
    fn clone(&self) -> Self {
        self.to_owned()
    }
}
