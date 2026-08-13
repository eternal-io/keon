#![allow(clippy::unnecessary_unwrap)]
use crate::{value::*, PrivateMethod};
#[cfg(feature = "alloc")]
use alloc::{collections::VecDeque, string::String, vec::Vec};
use core::{
    fmt::{self, Display, Write},
    ops::{Deref, DerefMut},
};

mod ser_concr;
#[cfg(feature = "alloc")]
mod ser_value;

#[cfg(all(feature = "alloc", feature = "ecow"))]
type EcoString = ecow::EcoString;
#[cfg(all(feature = "alloc", not(feature = "ecow")))]
type EcoString = alloc::string::String;

#[cfg(feature = "alloc")]
pub fn stringify<T: Serialize>(value: &T) -> Result<String, fmt::Error> {
    let mut stringified = String::with_capacity(256);
    Serializer::new(&mut stringified).serialize(value)?;
    Ok(stringified)
}

#[cfg(feature = "alloc")]
pub fn stringify_pretty<T: Serialize>(value: &T) -> Result<String, fmt::Error> {
    let mut stringified = String::with_capacity(256);
    Serializer::new_pretty(&mut stringified).serialize(value)?;
    Ok(stringified)
}

//==================================================================================================

#[expect(private_interfaces, reason = "Sealed")]
pub trait Serialize {
    #[doc(hidden)]
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut Serializer<Impl>, _: PrivateMethod) -> fmt::Result;
}

#[expect(private_bounds, reason = "Sealed")]
pub trait SerializerImpl: SerializerImplDetail {}

trait SerializerImplDetail {
    #[cfg(feature = "alloc")]
    fn push_stringified(&mut self, stringified: EcoString) -> fmt::Result;

    fn push_bool(&mut self, b: bool) -> fmt::Result;
    fn push_char(&mut self, ch: char) -> fmt::Result;
    fn push_number(&mut self, num: &Number) -> fmt::Result;
    fn push_str(&mut self, s: &str) -> fmt::Result;
    fn push_display<T: ?Sized + Display>(&mut self, value: &T) -> fmt::Result;
    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result;
    fn push_scalar(&mut self, scalar: &Scalar) -> fmt::Result {
        match scalar {
            Scalar::Char(ch) => self.push_char(*ch),
            Scalar::Number(num) => self.push_number(num),
        }
    }

    fn push_i8(&mut self, n: i8) -> fmt::Result;
    fn push_i16(&mut self, n: i16) -> fmt::Result;
    fn push_i32(&mut self, n: i32) -> fmt::Result;
    fn push_i64(&mut self, n: i64) -> fmt::Result;
    fn push_i128(&mut self, n: i128) -> fmt::Result;
    fn push_u8(&mut self, n: u8) -> fmt::Result;
    fn push_u16(&mut self, n: u16) -> fmt::Result;
    fn push_u32(&mut self, n: u32) -> fmt::Result;
    fn push_u64(&mut self, n: u64) -> fmt::Result;
    fn push_u128(&mut self, n: u128) -> fmt::Result;
    fn push_f32(&mut self, n: f32) -> fmt::Result;
    fn push_f64(&mut self, n: f64) -> fmt::Result;

    fn push_range_full(&mut self) -> fmt::Result;
    fn push_range_to(&mut self, end: &Scalar) -> fmt::Result;
    fn push_range_to_inclusive(&mut self, end: &Scalar) -> fmt::Result;
    fn push_range_from(&mut self, start: &Scalar) -> fmt::Result;
    fn push_range(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result;
    fn push_range_inclusive(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result;

    fn push_maybe_begin(&mut self) -> fmt::Result;
    fn push_maybe_end(&mut self) -> fmt::Result;

    fn push_array_begin(&mut self) -> fmt::Result;
    fn push_array_end(&mut self) -> fmt::Result;

    fn push_unit(&mut self) -> fmt::Result;
    fn push_unit_struct(&mut self, name: Option<&Ident>) -> fmt::Result;
    fn push_unit_variant(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result;

    fn push_tuple_begin(&mut self) -> fmt::Result;
    fn push_tuple_struct_begin(&mut self, name: Option<&Ident>) -> fmt::Result;
    fn push_tuple_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result;
    fn push_tuple_like_end(&mut self) -> fmt::Result;

    fn push_map_begin(&mut self) -> fmt::Result;
    fn push_map_struct_begin(&mut self, name: Option<&Ident>, intercept_range: bool) -> fmt::Result;
    fn push_map_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result;
    fn push_map_like_end(&mut self) -> fmt::Result;

    fn push_newtype_begin(&mut self, name: Option<&Ident>) -> fmt::Result;
    fn push_newtype_end(&mut self) -> fmt::Result;

    fn push_identifier(&mut self, field: &Ident) -> fmt::Result;

    fn hint_map_key(&mut self);
    fn push_fat_arrow(&mut self) -> fmt::Result;
    fn push_colon(&mut self) -> fmt::Result;
    fn push_comma(&mut self) -> fmt::Result;

    fn semicolon(&mut self) -> fmt::Result;
}

pub struct Serializer<Impl> {
    ser: Impl,
    ttl: u16,
}

impl<Impl> Deref for Serializer<Impl> {
    type Target = Impl;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ser
    }
}

impl<Impl> DerefMut for Serializer<Impl> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ser
    }
}

impl<Impl> Serializer<Impl> {
    #[inline]
    pub fn corrupted(&self) -> bool {
        self.ttl == 0
    }
}

impl<W: Write> Serializer<FastImpl<W>> {
    pub fn new(dst: W) -> Self {
        Self::with_flags(dst, Flags::SMALLER_OMIT_NAMES)
    }

    pub fn with_flags(dst: W, flags: Flags) -> Self {
        Self::with_flags_and_limit(dst, flags, 160)
    }

    pub fn with_flags_and_limit(dst: W, flags: Flags, recursion_limit: u16) -> Self {
        Self {
            ser: FastImpl::new(dst, flags),
            ttl: recursion_limit,
        }
    }
}

#[cfg(feature = "alloc")]
impl<W: Write> Serializer<PrettyImpl<W>> {
    pub fn new_pretty(dst: W) -> Self {
        Self::with_flags_pretty(dst, Flags::DEFAULT)
    }

    pub fn with_flags_pretty(dst: W, flags: Flags) -> Self {
        Self::with_flags_and_limit_pretty(dst, flags, 160)
    }

    pub fn with_flags_and_limit_pretty(dst: W, flags: Flags, recursion_limit: u16) -> Self {
        Self {
            ser: PrettyImpl::new(dst, flags),
            ttl: recursion_limit,
        }
    }
}

impl<Impl: SerializerImpl> Serializer<Impl> {
    pub fn serialize<T>(&mut self, value: &T) -> fmt::Result
    where
        T: ?Sized + Serialize,
    {
        self.serialize_inner(value)?;
        self.semicolon()
    }

    pub fn serialize_many<T, I>(&mut self, values: I) -> fmt::Result
    where
        T: ?Sized + Serialize,
        I: IntoIterator<Item: AsRef<T>>,
    {
        values.into_iter().try_for_each(|value| self.serialize(value.as_ref()))
    }

    fn serialize_inner<T: ?Sized + Serialize>(&mut self, value: &T) -> fmt::Result {
        if self.ttl == 0 {
            return Err(fmt::Error);
        }
        self.ttl -= 1;
        value.serialize_with(self, PrivateMethod)?;
        self.ttl += 1;
        Ok(())
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Flags: u16 {
        const EXPAND_DEPTH_PLUS_1   = 1 << 0;
        const EXPAND_DEPTH_PLUS_2   = 1 << 1;
        const EXPAND_DEPTH_PLUS_4   = 1 << 2;

        const MAX_WIDTH_PLUS_8      = 1 << 3;
        const MAX_WIDTH_PLUS_16     = 1 << 4;
        const MAX_WIDTH_PLUS_32     = 1 << 5;
        const MAX_WIDTH_PLUS_64     = 1 << 6;

        const COMPACTIZED_MAP_KEY   = 1 << 7;

        const OMIT_STRUCT_NAME      = 1 << 8;
        const OMIT_NEWTYPE_NAME     = 1 << 9;
        const OMIT_ENUM_NAME        = 1 << 10;
        const IMPLICIT_NEWTYPE      = 1 << 11;
        const IMPLICIT_VARIANT      = 1 << 12;

        const WRITE_INTEGER_SUFFIX  = 1 << 13;
        const WRITE_FLOAT_SUFFIX    = 1 << 14;

        const HARD_TAB              = 1 << 15;

        const EXPAND_DEPTH_EQUAL_7
            = Self::EXPAND_DEPTH_PLUS_1.bits()
            | Self::EXPAND_DEPTH_PLUS_2.bits()
            | Self::EXPAND_DEPTH_PLUS_4.bits()
            ;
        const MAX_WIDTH_EQUAL_120
            = Self::MAX_WIDTH_PLUS_8.bits()
            | Self::MAX_WIDTH_PLUS_16.bits()
            | Self::MAX_WIDTH_PLUS_32.bits()
            | Self::MAX_WIDTH_PLUS_64.bits()
            ;
        const OMIT_NOMINAL_NAMES
            = Self::OMIT_STRUCT_NAME.bits()
            | Self::OMIT_NEWTYPE_NAME.bits()
            | Self::OMIT_ENUM_NAME.bits()
            ;
        const IMPLICIT_STRUCTURALS
            = Self::IMPLICIT_NEWTYPE.bits()
            | Self::IMPLICIT_VARIANT.bits()
            ;
        const WRITE_NUMBER_SUFFIX
            = Self::WRITE_INTEGER_SUFFIX.bits()
            | Self::WRITE_FLOAT_SUFFIX.bits()
            ;
        const DEFAULT
            = Self::EXPAND_DEPTH_PLUS_1.bits()
            | Self::MAX_WIDTH_EQUAL_120.bits()
            | Self::COMPACTIZED_MAP_KEY.bits()
            ;
        const SMALLER_OMIT_NAMES
            = Self::EXPAND_DEPTH_PLUS_1.bits()
            | Self::MAX_WIDTH_EQUAL_120.bits()
            | Self::COMPACTIZED_MAP_KEY.bits()
            | Self::OMIT_NOMINAL_NAMES.bits()
            | Self::HARD_TAB.bits()
            ;
        const SMALLER_IMPLICIT_ALL
            = Self::EXPAND_DEPTH_PLUS_1.bits()
            | Self::MAX_WIDTH_EQUAL_120.bits()
            | Self::COMPACTIZED_MAP_KEY.bits()
            | Self::OMIT_NOMINAL_NAMES.bits()
            | Self::IMPLICIT_STRUCTURALS.bits()
            | Self::HARD_TAB.bits()
            ;
    }
}

impl Flags {
    pub fn expand_depth(&self) -> usize {
        self.intersection(Self::EXPAND_DEPTH_EQUAL_7).bits() as usize
    }
    pub fn max_width(&self) -> usize {
        self.intersection(Self::MAX_WIDTH_EQUAL_120).bits() as usize
    }
    pub fn compactized_map_key(&self) -> bool {
        self.contains(Self::COMPACTIZED_MAP_KEY)
    }

    pub fn omit_struct_name(&self) -> bool {
        self.contains(Self::OMIT_STRUCT_NAME)
    }
    pub fn omit_newtype_name(&self) -> bool {
        self.contains(Self::OMIT_NEWTYPE_NAME)
    }
    pub fn omit_enum_name(&self) -> bool {
        self.contains(Self::OMIT_ENUM_NAME)
    }
    pub fn implicit_newtype(&self) -> bool {
        self.contains(Self::IMPLICIT_NEWTYPE)
    }
    pub fn implicit_variant(&self) -> bool {
        self.contains(Self::IMPLICIT_VARIANT)
    }

    pub fn write_integer_suffix(&self) -> bool {
        self.contains(Self::WRITE_INTEGER_SUFFIX)
    }
    pub fn write_float_suffix(&self) -> bool {
        self.contains(Self::WRITE_FLOAT_SUFFIX)
    }
    pub fn hard_tab(&self) -> bool {
        self.contains(Self::HARD_TAB)
    }
}

//==================================================================================================

pub struct FastImpl<W> {
    dst: W,
    flags: Flags,
    interceptor: RangeInterceptor,
}

impl<W> FastImpl<W> {
    fn new(dst: W, flags: Flags) -> Self {
        Self {
            dst,
            flags,
            interceptor: Default::default(),
        }
    }
}

impl<W: Write> FastImpl<W> {
    fn clear_range_intercept(&mut self) -> fmt::Result {
        if self.interceptor.is_active() {
            self.clear_range_intercept_cold()?;
        }
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn clear_range_intercept_cold(&mut self) -> fmt::Result {
        core::mem::take(&mut self.interceptor).clear(self)
    }

    fn intercept_range_bound<T, F>(&mut self, scalar: T, or_else: F) -> fmt::Result
    where
        T: Into<Scalar>,
        F: FnOnce(&mut Self, T) -> fmt::Result,
    {
        if self.interceptor.is_active() {
            self.interceptor.push_value(scalar);
            Ok(())
        } else {
            or_else(self, scalar)
        }
    }
}

macro_rules! push_concr_number_fast {
    ($method:ident, $ty:ty, $suff:ident, $write_fn:ident) => {
        fn $method(&mut self, n: $ty) -> fmt::Result {
            self.intercept_range_bound(n, |ser, n| {
                $write_fn(&mut ser.dst, n.into())?;
                write_number_suffix(&mut ser.dst, NumberSuffix::$suff, ser.flags)
            })
        }
    };
}

impl<W: Write> SerializerImpl for FastImpl<W> {}

impl<W: Write> SerializerImplDetail for FastImpl<W> {
    #[cfg(feature = "alloc")]
    fn push_stringified(&mut self, _stringified: EcoString) -> fmt::Result {
        panic!("FastImpl does not rely on alloc")
    }

    fn push_bool(&mut self, b: bool) -> fmt::Result {
        self.clear_range_intercept()?;
        write_bool(&mut self.dst, b)
    }
    fn push_char(&mut self, ch: char) -> fmt::Result {
        self.intercept_range_bound(ch, |ser, ch| write_quoted_char(&mut ser.dst, ch))
    }
    fn push_number(&mut self, num: &Number) -> fmt::Result {
        self.intercept_range_bound(num, |ser, num| write_number(&mut ser.dst, num, ser.flags))
    }
    fn push_str(&mut self, s: &str) -> fmt::Result {
        self.clear_range_intercept()?;
        write_quoted_string(&mut self.dst, s)
    }
    fn push_display<T: ?Sized + Display>(&mut self, value: &T) -> fmt::Result {
        self.clear_range_intercept()?;
        write!(EscapedWriter(&mut self.dst), r#""{}""#, value)
    }
    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
        self.clear_range_intercept()?;
        write_quoted_byte_string(&mut self.dst, bytes)
    }

    push_concr_number_fast!(push_i8, i8, Int8, write_i64);
    push_concr_number_fast!(push_i16, i16, Int16, write_i64);
    push_concr_number_fast!(push_i32, i32, Int32, write_i64);
    push_concr_number_fast!(push_i64, i64, Int64, write_i64);
    push_concr_number_fast!(push_i128, i128, Int128, write_i128);
    push_concr_number_fast!(push_u8, u8, UInt8, write_u64);
    push_concr_number_fast!(push_u16, u16, UInt16, write_u64);
    push_concr_number_fast!(push_u32, u32, UInt32, write_u64);
    push_concr_number_fast!(push_u64, u64, UInt64, write_u64);
    push_concr_number_fast!(push_u128, u128, UInt128, write_u128);
    push_concr_number_fast!(push_f32, f32, Float32, write_f32);
    push_concr_number_fast!(push_f64, f64, Float64, write_f64);

    fn push_range_full(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("..")
    }
    fn push_range_to(&mut self, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("..")?;
        write_scalar(&mut self.dst, end, self.flags)
    }
    fn push_range_to_inclusive(&mut self, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("..=")?;
        write_scalar(&mut self.dst, end, self.flags)
    }
    fn push_range_from(&mut self, start: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        write_scalar(&mut self.dst, start, self.flags)?;
        self.dst.write_str("..")
    }
    fn push_range(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        write_scalar(&mut self.dst, start, self.flags)?;
        self.dst.write_str("..")?;
        write_scalar(&mut self.dst, end, self.flags)
    }
    fn push_range_inclusive(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        write_scalar(&mut self.dst, start, self.flags)?;
        self.dst.write_str("..=")?;
        write_scalar(&mut self.dst, end, self.flags)
    }

    fn push_maybe_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("?")
    }
    fn push_maybe_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());

        Ok(())
    }

    fn push_array_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("[")
    }
    fn push_array_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());

        self.dst.write_str("]")
    }

    fn push_unit(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("()")
    }
    fn push_unit_struct(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        if let Some("RangeFull") = name.map(Ident::as_str) {
            self.push_range_full()
        } else {
            write_struct_name(&mut self.dst, name, self.flags)
        }
    }
    fn push_unit_variant(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        self.clear_range_intercept()?;
        write_variant_name(&mut self.dst, name, variant, self.flags)
    }

    fn push_tuple_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("(")
    }
    fn push_tuple_struct_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        write_struct_name(&mut self.dst, name, self.flags)?;
        self.dst.write_str("(")
    }
    fn push_tuple_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        self.clear_range_intercept()?;
        write_variant_name(&mut self.dst, name, variant, self.flags)?;
        self.dst.write_str("(")
    }
    fn push_tuple_like_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());

        self.dst.write_str(")")
    }

    fn push_map_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("{")
    }
    fn push_map_struct_begin(&mut self, name: Option<&Ident>, intercept_range: bool) -> fmt::Result {
        self.clear_range_intercept()?;
        if self.interceptor.begin(name, intercept_range) {
            return Ok(());
        }
        write_struct_name(&mut self.dst, name, self.flags)?;
        self.dst.write_str("{")
    }
    fn push_map_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        self.clear_range_intercept()?;
        write_variant_name(&mut self.dst, name, variant, self.flags)?;
        self.dst.write_str("{")
    }
    fn push_map_like_end(&mut self) -> fmt::Result {
        if core::mem::take(&mut self.interceptor).finish(self)? {
            return Ok(());
        }
        self.dst.write_str("}")
    }

    fn push_newtype_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        write_newtype_name(&mut self.dst, name, self.flags)
    }
    fn push_newtype_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());

        Ok(())
    }

    fn push_identifier(&mut self, field: &Ident) -> fmt::Result {
        match self.interceptor.push_field(field) {
            RangeInterceptorStatus::Inactive => (),
            RangeInterceptorStatus::Accepted => return Ok(()),
            RangeInterceptorStatus::Rejected => self.clear_range_intercept_cold()?,
        }
        self.dst.write_str(field)
    }

    fn hint_map_key(&mut self) {}
    fn push_fat_arrow(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());

        self.dst.write_str("=>")
    }
    fn push_colon(&mut self) -> fmt::Result {
        if self.interceptor.is_active() {
            return Ok(());
        }
        self.dst.write_str(":")
    }
    fn push_comma(&mut self) -> fmt::Result {
        if self.interceptor.is_active() {
            return Ok(());
        }
        self.dst.write_str(",")
    }

    fn semicolon(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());

        self.dst.write_str(";\n")
    }
}

//==================================================================================================

#[cfg(feature = "alloc")]
pub struct PrettyImpl<W> {
    dst: W,
    flags: Flags,
    interceptor: RangeInterceptor,
    line_buffer: LineBuffer<Compound>,
    inside_map_key: u16,
}

#[cfg(feature = "alloc")]
impl<W> PrettyImpl<W> {
    fn new(dst: W, flags: Flags) -> Self {
        Self {
            dst,
            flags,
            interceptor: Default::default(),
            line_buffer: LineBuffer::new(),
            inside_map_key: 0,
        }
    }
}

#[cfg(feature = "alloc")]
enum Compound {
    Maybe,
    Array,
    Tuple,
    MapWritingKey,
    MapWritingValue,
    Newtype(EcoString),
    NominalTuple(EcoString),
    StructWritingKey(EcoString),
    StructWritingValue(EcoString),
}

#[cfg(feature = "alloc")]
impl<W: Write> PrettyImpl<W> {
    #[inline(always)]
    fn clear_range_intercept(&mut self) -> fmt::Result {
        if self.interceptor.is_active() {
            self.clear_range_intercept_cold()?;
        }
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn clear_range_intercept_cold(&mut self) -> fmt::Result {
        core::mem::take(&mut self.interceptor).clear(self)
    }

    fn intercept_range_bound<T, F>(&mut self, scalar: T, or_writing: F) -> fmt::Result
    where
        T: Into<Scalar>,
        F: FnOnce(&mut EcoString, T, Flags) -> fmt::Result,
    {
        if self.interceptor.is_active() {
            self.interceptor.push_value(scalar);
            Ok(())
        } else {
            self.push_stringifying(|dst, flags| or_writing(dst, scalar, flags))
        }
    }

    fn push_stringifying<F>(&mut self, writing: F) -> fmt::Result
    where
        F: FnOnce(&mut EcoString, Flags) -> fmt::Result,
    {
        self.push_stringified(writing_string(|dst| writing(dst, self.flags))?)
    }

    fn push_compound(&mut self, compound: Compound) -> fmt::Result {
        self.line_buffer.push_group(compound);
        Ok(())
    }

    fn pop_compound(&mut self) -> fmt::Result {
        let (compd, maybe_frags) = self.line_buffer.pop_group().expect("compound");
        if maybe_frags.is_some() {
            let mut frags = maybe_frags.unwrap().peekable();
            let stringified = writing_string(move |dst| {
                match compd {
                    Compound::MapWritingValue => panic!(),
                    Compound::MapWritingKey => {
                        if frags.peek().is_none() {
                            dst.write_str("{}")?;
                        } else {
                            dst.write_str("{ ")?;
                            while let Some(key) = frags.next() {
                                let value = frags.next().expect("key value");
                                dst.write_str(&key)?;
                                dst.write_str(" => ")?;
                                dst.write_str(&value)?;
                                if frags.peek().is_some() {
                                    dst.write_str(", ")?;
                                }
                            }
                            dst.write_str(" }")?;
                        }
                    }
                    Compound::StructWritingValue(_) => panic!(),
                    Compound::StructWritingKey(header) => {
                        dst.write_str(&header)?;
                        dst.write_str(" ")?;
                        if frags.peek().is_none() {
                            dst.write_str("{}")?;
                        } else {
                            dst.write_str("{ ")?;
                            while let Some(key) = frags.next() {
                                let value = frags.next().expect("key value");
                                dst.write_str(&key)?;
                                dst.write_str(": ")?;
                                dst.write_str(&value)?;
                                if frags.peek().is_some() {
                                    dst.write_str(", ")?;
                                }
                            }
                            dst.write_str(" }")?;
                        }
                    }
                    Compound::Tuple | Compound::NominalTuple(_) => {
                        if let Compound::NominalTuple(header) = compd {
                            dst.write_str(&header)?;
                        }
                        dst.write_str("(")?;
                        while let Some(value) = frags.next() {
                            dst.write_str(&value)?;
                            if frags.peek().is_some() {
                                dst.write_str(", ")?;
                            }
                        }
                        dst.write_str(")")?;
                    }
                    Compound::Array => {
                        dst.write_str("[")?;
                        while let Some(value) = frags.next() {
                            dst.write_str(&value)?;
                            if frags.peek().is_some() {
                                dst.write_str(", ")?;
                            }
                        }
                        dst.write_str("]")?;
                    }
                    Compound::Maybe => {
                        if let Some(frag) = frags.next() {
                            dst.write_str("? ")?;
                            dst.write_str(&frag)?;
                        } else {
                            dst.write_str("?")?
                        }
                        debug_assert!(frags.next().is_none());
                    }
                    Compound::Newtype(header) => {
                        dst.write_str(&header)?; // space included
                        dst.write_str(&frags.next().expect("newtype body"))?;
                        debug_assert!(frags.next().is_none());
                    }
                }
                Ok(())
            })?;
            self.push_stringified(stringified)?;
        } else {
            drop(maybe_frags);
            'write_term: {
                let term = match compd {
                    Compound::Array => "]",
                    Compound::MapWritingKey | Compound::StructWritingKey(_) => "}",
                    Compound::MapWritingValue | Compound::StructWritingValue(_) => panic!(),
                    Compound::Tuple | Compound::NominalTuple(_) => ")",
                    Compound::Maybe | Compound::Newtype(_) => break 'write_term,
                };
                write_indent(&mut self.dst, self.line_buffer.groups_count(), self.flags.hard_tab())?;
                self.dst.write_str(term)?;
            }
        }

        if let Some(compd) = self.line_buffer.last_group() {
            if matches!(
                compd,
                Compound::Array
                    | Compound::Tuple
                    | Compound::NominalTuple(_)
                    | Compound::MapWritingValue
                    | Compound::StructWritingValue(_)
            ) {
                self.flip_next_writing_key_or_value();
                self.dst.write_str(",\n")?;
            } else if matches!(compd, Compound::MapWritingKey) {
                self.flip_next_writing_key_or_value();
                self.dst.write_str(" => ")?;
            } else if matches!(compd, Compound::StructWritingKey(_)) {
                self.flip_next_writing_key_or_value();
                self.dst.write_str(": ")?;
            }
        }

        Ok(())
    }

    fn flip_next_writing_key_or_value(&mut self) {
        let compd = self.line_buffer.last_group_mut().expect("compound");
        match compd {
            Compound::MapWritingKey => *compd = Compound::MapWritingValue,
            Compound::MapWritingValue => *compd = Compound::MapWritingKey,
            Compound::StructWritingKey(header) => *compd = Compound::StructWritingValue(core::mem::take(header)),
            Compound::StructWritingValue(header) => *compd = Compound::StructWritingKey(core::mem::take(header)),
            _ => (),
        }
    }
}

#[cfg(feature = "alloc")]
fn writing_string<F>(f: F) -> Result<EcoString, fmt::Error>
where
    F: FnOnce(&mut EcoString) -> fmt::Result,
{
    let mut stringified = EcoString::new();
    f(&mut stringified)?;
    Ok(stringified)
}

#[cfg(feature = "alloc")]
macro_rules! push_concr_number_pretty {
    ($method:ident, $ty:ty, $suff:ident, $write_fn:ident) => {
        fn $method(&mut self, n: $ty) -> fmt::Result {
            self.intercept_range_bound(n, |dst, n, flags| {
                $write_fn(dst, n.into())?;
                write_number_suffix(dst, NumberSuffix::$suff, flags)
            })
        }
    };
}

#[cfg(feature = "alloc")]
impl<W: Write> SerializerImpl for PrettyImpl<W> {}

#[cfg(feature = "alloc")]
impl<W: Write> SerializerImplDetail for PrettyImpl<W> {
    fn push_stringified(&mut self, stringified: EcoString) -> fmt::Result {
        let dst = &mut self.dst;
        let depth = self.line_buffer.groups_count();
        let hard_tab = self.flags.hard_tab();
        let next_to_key_value_separator = matches!(
            self.line_buffer.last_group(),
            Some(Compound::MapWritingValue | Compound::StructWritingValue(_))
        );
        let committed = if let Some(committed) = self.line_buffer.push(stringified) {
            committed
        } else {
            let expand_bcuz_depth = self.line_buffer.groups_count() <= self.flags.expand_depth();
            let expand_bcuz_line_width = self.line_buffer.accumulated_width > self.flags.max_width();
            let not_expand_bcuz_compact_map_key = self.flags.compactized_map_key() && self.inside_map_key > 0;
            let not_expand_bcuz_unnecessary = matches!(
                self.line_buffer.last_group(),
                Some(Compound::Maybe | Compound::Newtype(_))
            );
            // Pushing into an expanded group shall return Some(_) and not reach this branch.
            if (expand_bcuz_depth || expand_bcuz_line_width)
                && !not_expand_bcuz_compact_map_key
                && !not_expand_bcuz_unnecessary
            {
                let (compd, mut frags) = self.line_buffer.expand_group().unwrap();
                if !next_to_key_value_separator {
                    write_indent(dst, depth - 1, hard_tab)?;
                }
                match compd {
                    Compound::MapWritingKey | Compound::MapWritingValue => {
                        dst.write_str("{\n")?;
                        while let Some(key) = frags.next() {
                            write_indent(dst, depth, hard_tab)?;
                            dst.write_str(&key)?;
                            dst.write_str(" => ")?;
                            if let Some(value) = frags.next() {
                                dst.write_str(&value)?;
                                dst.write_str(",\n")?;
                            }
                        }
                        drop(frags);
                        self.flip_next_writing_key_or_value();
                    }
                    Compound::StructWritingKey(header) | Compound::StructWritingValue(header) => {
                        dst.write_str(header)?;
                        dst.write_str(" {\n")?;
                        while let Some(key) = frags.next() {
                            write_indent(dst, depth, hard_tab)?;
                            dst.write_str(&key)?;
                            dst.write_str(": ")?;
                            if let Some(value) = frags.next() {
                                dst.write_str(&value)?;
                                dst.write_str(",\n")?;
                            }
                        }
                        drop(frags);
                        self.flip_next_writing_key_or_value();
                    }
                    Compound::Tuple | Compound::NominalTuple(_) => {
                        if let Compound::NominalTuple(header) = compd {
                            dst.write_str(header)?;
                        }
                        dst.write_str("(\n")?;
                        for value in frags {
                            write_indent(dst, depth, hard_tab)?;
                            dst.write_str(&value)?;
                            dst.write_str(",\n")?;
                        }
                    }
                    Compound::Array => {
                        dst.write_str("[\n")?;
                        for value in frags {
                            write_indent(dst, depth, hard_tab)?;
                            dst.write_str(&value)?;
                            dst.write_str(",\n")?;
                        }
                    }
                    Compound::Maybe | Compound::Newtype(_) => unreachable!(),
                }
            }
            return Ok(());
        };

        if !next_to_key_value_separator {
            write_indent(dst, depth, hard_tab)?;
        }
        dst.write_str(&committed)?;

        if let Some(compd) = self.line_buffer.last_group() {
            if matches!(
                compd,
                Compound::Array
                    | Compound::Tuple
                    | Compound::NominalTuple(_)
                    | Compound::MapWritingValue
                    | Compound::StructWritingValue(_)
            ) {
                self.flip_next_writing_key_or_value();
                self.dst.write_str(",\n")?;
            } else if matches!(compd, Compound::MapWritingKey) {
                self.flip_next_writing_key_or_value();
                self.dst.write_str(" => ")?;
            } else if matches!(compd, Compound::StructWritingKey(_)) {
                self.flip_next_writing_key_or_value();
                self.dst.write_str(": ")?;
            }
        }

        Ok(())
    }

    fn push_bool(&mut self, b: bool) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, _flags| write_bool(dst, b))
    }
    fn push_char(&mut self, ch: char) -> fmt::Result {
        self.intercept_range_bound(ch, |dst, ch, _flags| write_quoted_char(dst, ch))
    }
    fn push_number(&mut self, num: &Number) -> fmt::Result {
        self.intercept_range_bound(num, write_number)
    }
    fn push_str(&mut self, s: &str) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, _flags| write_quoted_string(dst, s))
    }
    fn push_display<T: ?Sized + Display>(&mut self, value: &T) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, _flags| write!(EscapedWriter(dst), r#""{}""#, value))
    }
    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, _flags| write_quoted_byte_string(dst, bytes))
    }

    push_concr_number_pretty!(push_i8, i8, Int8, write_i64);
    push_concr_number_pretty!(push_i16, i16, Int16, write_i64);
    push_concr_number_pretty!(push_i32, i32, Int32, write_i64);
    push_concr_number_pretty!(push_i64, i64, Int64, write_i64);
    push_concr_number_pretty!(push_i128, i128, Int128, write_i128);
    push_concr_number_pretty!(push_u8, u8, UInt8, write_u64);
    push_concr_number_pretty!(push_u16, u16, UInt16, write_u64);
    push_concr_number_pretty!(push_u32, u32, UInt32, write_u64);
    push_concr_number_pretty!(push_u64, u64, UInt64, write_u64);
    push_concr_number_pretty!(push_u128, u128, UInt128, write_u128);
    push_concr_number_pretty!(push_f32, f32, Float32, write_f32);
    push_concr_number_pretty!(push_f64, f64, Float64, write_f64);

    fn push_range_full(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringified(EcoString::from(".."))
    }
    fn push_range_to(&mut self, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, flags| {
            dst.write_str("..")?;
            write_scalar(dst, end, flags)
        })
    }
    fn push_range_to_inclusive(&mut self, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, flags| {
            dst.write_str("..=")?;
            write_scalar(dst, end, flags)
        })
    }
    fn push_range_from(&mut self, start: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, flags| {
            write_scalar(dst, start, flags)?;
            dst.write_str("..")
        })
    }
    fn push_range(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, flags| {
            write_scalar(dst, start, flags)?;
            dst.write_str("..")?;
            write_scalar(dst, end, flags)
        })
    }
    fn push_range_inclusive(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, flags| {
            write_scalar(dst, start, flags)?;
            dst.write_str("..=")?;
            write_scalar(dst, end, flags)
        })
    }

    fn push_maybe_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::Maybe)
    }
    fn push_maybe_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());
        debug_assert!(matches!(self.line_buffer.last_group(), Some(Compound::Maybe)));
        self.pop_compound()
    }

    fn push_array_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::Array)
    }
    fn push_array_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());
        debug_assert!(matches!(self.line_buffer.last_group(), Some(Compound::Array)));
        self.pop_compound()
    }

    fn push_unit(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringified(EcoString::from("()"))
    }
    fn push_unit_struct(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        if let Some("RangeFull") = name.map(Ident::as_str) {
            self.push_range_full()
        } else {
            self.push_stringifying(|dst, flags| write_struct_name(dst, name, flags))
        }
    }
    fn push_unit_variant(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_stringifying(|dst, flags| write_variant_name(dst, name, variant, flags))
    }

    fn push_tuple_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::Tuple)
    }
    fn push_tuple_struct_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::NominalTuple(writing_string(|dst| {
            write_struct_name(dst, name, self.flags)
        })?))
    }
    fn push_tuple_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::NominalTuple(writing_string(|dst| {
            write_variant_name(dst, name, variant, self.flags)
        })?))
    }
    fn push_tuple_like_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());
        debug_assert!(matches!(
            self.line_buffer.last_group(),
            Some(Compound::Tuple | Compound::NominalTuple(_))
        ));
        self.pop_compound()
    }

    fn push_map_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::MapWritingKey)
    }
    fn push_map_struct_begin(&mut self, name: Option<&Ident>, intercept_range: bool) -> fmt::Result {
        self.clear_range_intercept()?;
        if self.interceptor.begin(name, intercept_range) {
            return Ok(());
        }
        self.push_compound(Compound::StructWritingKey(writing_string(|dst| {
            write_struct_name(dst, name, self.flags)
        })?))
    }
    fn push_map_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::StructWritingKey(writing_string(|dst| {
            write_variant_name(dst, name, variant, self.flags)
        })?))
    }
    fn push_map_like_end(&mut self) -> fmt::Result {
        if core::mem::take(&mut self.interceptor).finish(self)? {
            return Ok(());
        }
        debug_assert!(matches!(
            self.line_buffer.last_group(),
            Some(Compound::MapWritingKey | Compound::StructWritingKey(_))
        ));
        self.pop_compound()
    }

    fn push_newtype_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        self.push_compound(Compound::Newtype(writing_string(|dst| {
            write_newtype_name(dst, name, self.flags)
        })?))
    }
    fn push_newtype_end(&mut self) -> fmt::Result {
        debug_assert!(!self.interceptor.is_active());
        debug_assert!(matches!(self.line_buffer.last_group(), Some(Compound::Newtype(_))));
        self.pop_compound()
    }

    fn push_identifier(&mut self, field: &Ident) -> fmt::Result {
        match self.interceptor.push_field(field) {
            RangeInterceptorStatus::Inactive => (),
            RangeInterceptorStatus::Accepted => return Ok(()),
            RangeInterceptorStatus::Rejected => self.clear_range_intercept_cold()?,
        }
        self.push_stringified(EcoString::from(field.as_str()))
    }

    #[inline(always)]
    fn hint_map_key(&mut self) {
        self.inside_map_key += 1;
    }
    #[inline(always)]
    fn push_fat_arrow(&mut self) -> fmt::Result {
        self.inside_map_key -= 1;
        Ok(())
    }
    #[inline(always)]
    fn push_colon(&mut self) -> fmt::Result {
        Ok(())
    }
    #[inline(always)]
    fn push_comma(&mut self) -> fmt::Result {
        Ok(())
    }

    fn semicolon(&mut self) -> fmt::Result {
        self.dst.write_str(";\n")
    }
}

//==================================================================================================

#[derive(Default)]
struct RangeInterceptor {
    range_type: Option<RangeType>,
    next_field: NextField,
    field_start: Option<Option<Scalar>>,
    field_end: Option<Option<Scalar>>,
}

#[derive(Default)]
enum NextField {
    #[default]
    Unknown,
    Start,
    End,
}

enum RangeType {
    RangeTo,
    RangeToInclusive,
    RangeFrom,
    Range,
    RangeInclusive,
}

enum RangeInterceptorStatus {
    Inactive,
    Accepted,
    Rejected, // must clear
}

impl RangeInterceptor {
    fn is_active(&self) -> bool {
        self.range_type.is_some()
    }

    fn push_value(&mut self, scalar: impl Into<Scalar>) {
        match self.next_field {
            NextField::Unknown => panic!(),
            NextField::Start => self.field_start = Some(Some(scalar.into())),
            NextField::End => self.field_end = Some(Some(scalar.into())),
        }
    }

    fn push_field(&mut self, field: &Ident) -> RangeInterceptorStatus {
        let Some(ref typ) = self.range_type else {
            return RangeInterceptorStatus::Inactive;
        };

        if field.as_str() == "start"
            && matches!(typ, RangeType::Range | RangeType::RangeInclusive | RangeType::RangeFrom)
        {
            self.next_field = NextField::Start;
            self.field_start = Some(None);
        } else if field.as_str() == "end"
            && matches!(
                typ,
                RangeType::Range | RangeType::RangeInclusive | RangeType::RangeTo | RangeType::RangeToInclusive
            )
        {
            self.next_field = NextField::End;
            self.field_end = Some(None);
        } else {
            self.next_field = NextField::Unknown;
            return RangeInterceptorStatus::Rejected;
        }

        RangeInterceptorStatus::Accepted
    }

    fn begin(&mut self, name: Option<&Ident>, intercept_range: bool) -> bool {
        if name.is_none() || !intercept_range {
            return false;
        }
        let typ = match name.unwrap().as_str() {
            "RangeTo" => RangeType::RangeTo,
            "RangeToInclusive" => RangeType::RangeToInclusive,
            "RangeFrom" => RangeType::RangeFrom,
            "Range" => RangeType::Range,
            "RangeInclusive" => RangeType::RangeInclusive,
            _ => return false,
        };
        self.range_type = Some(typ);

        true
    }

    fn finish(self, ser: &mut impl SerializerImpl) -> Result<bool, fmt::Error> {
        if let Some(ref typ) = self.range_type {
            'range_type: {
                match typ {
                    RangeType::RangeTo => {
                        if let (None, Some(Some(end))) = (self.field_start, self.field_end) {
                            ser.push_range_to(&end)?;
                        } else {
                            break 'range_type;
                        }
                    }
                    RangeType::RangeToInclusive => {
                        if let (None, Some(Some(end))) = (self.field_start, self.field_end) {
                            ser.push_range_to_inclusive(&end)?;
                        } else {
                            break 'range_type;
                        }
                    }
                    RangeType::RangeFrom => {
                        if let (Some(Some(start)), None) = (self.field_start, self.field_end) {
                            ser.push_range_from(&start)?;
                        } else {
                            break 'range_type;
                        }
                    }
                    RangeType::Range => {
                        if let (Some(Some(start)), Some(Some(end))) = (self.field_start, self.field_end) {
                            ser.push_range(&start, &end)?;
                        } else {
                            break 'range_type;
                        }
                    }
                    RangeType::RangeInclusive => {
                        if let (Some(Some(start)), Some(Some(end))) = (self.field_start, self.field_end) {
                            ser.push_range_inclusive(&start, &end)?;
                        } else {
                            break 'range_type;
                        }
                    }
                }
                return Ok(true);
            }
            self.clear(ser)?;
        }
        Ok(false)
    }

    fn clear(self, ser: &mut impl SerializerImpl) -> fmt::Result {
        let Some(typ) = self.range_type else {
            return Ok(());
        };
        let name = match typ {
            RangeType::RangeTo => "RangeTo",
            RangeType::RangeToInclusive => "RangeToInclusive",
            RangeType::RangeFrom => "RangeFrom",
            RangeType::Range => "Range",
            RangeType::RangeInclusive => "RangeInclusive",
        };
        ser.push_map_struct_begin(Some(Ident::new_unchecked(name)), false)?;

        let (mut fst, mut snd) = (("start", self.field_start), ("end", self.field_end));
        let mut op = |(field, maybe_field_maybe_value)| {
            if let Some(maybe_scalar) = maybe_field_maybe_value {
                ser.push_identifier(Ident::new_unchecked(field))?;
                if let Some(scalar) = maybe_scalar {
                    ser.push_colon()?;
                    ser.push_scalar(&scalar)?;
                    ser.push_comma()?;
                }
            }
            Ok(())
        };
        if let NextField::Start = self.next_field {
            core::mem::swap(&mut fst, &mut snd);
        }
        op(fst)?;
        op(snd)?;
        Ok(())
    }
}

//------------------------------------------------------------------------------

#[cfg(feature = "alloc")]
struct LineBuffer<Group> {
    groups: Vec<(usize, Group)>,
    fragments: VecDeque<EcoString>,
    next_need_expand: usize,
    accumulated_width: usize,
}

#[cfg(feature = "alloc")]
impl<Group> LineBuffer<Group> {
    fn new() -> Self {
        Self {
            groups: Vec::new(),
            fragments: VecDeque::new(),
            next_need_expand: 0,
            accumulated_width: 0,
        }
    }

    fn groups_count(&self) -> usize {
        self.groups.len()
    }
    fn last_group(&self) -> Option<&Group> {
        Some(&self.groups.last()?.1)
    }
    fn last_group_mut(&mut self) -> Option<&mut Group> {
        Some(&mut self.groups.last_mut()?.1)
    }
    fn push_group(&mut self, group: Group) {
        self.groups.push((self.fragments.len(), group));
    }
    fn pop_group(&mut self) -> Option<(Group, Option<impl Iterator<Item = EcoString> + '_>)> {
        if self.next_need_expand < self.groups.len() {
            let (index, group) = self.groups.pop()?;
            Some((
                group,
                Some(
                    self.fragments
                        .drain(index..)
                        .inspect(|frag| self.accumulated_width -= frag.len()),
                ),
            ))
        } else {
            let (index, group) = self.groups.pop()?;
            self.next_need_expand = self.groups.len();
            debug_assert!(index == self.fragments.len());
            Some((group, None))
        }
    }
    fn expand_group(&mut self) -> Option<(&mut Group, impl Iterator<Item = EcoString> + '_)> {
        let (need_expand, compacts) = self.groups.get_mut(self.next_need_expand..)?.split_first_mut()?;
        self.next_need_expand += 1;
        let count = need_expand.0;
        let group = &mut need_expand.1;

        need_expand.0 -= count;
        for still_compact in compacts {
            still_compact.0 -= count;
        }

        Some((
            group,
            self.fragments
                .drain(..count)
                .inspect(|frag| self.accumulated_width -= frag.len()),
        ))
    }

    fn push(&mut self, frag: EcoString) -> Option<EcoString> {
        if self.next_need_expand < self.groups.len() {
            self.accumulated_width += frag.len();
            self.fragments.push_back(frag);
            None
        } else {
            Some(frag)
        }
    }
}

#[cfg(feature = "alloc")]
impl<Group> Drop for LineBuffer<Group> {
    fn drop(&mut self) {
        debug_assert!(self.groups.is_empty());
        debug_assert!(self.fragments.is_empty());
    }
}

//==================================================================================================

#[cfg(feature = "alloc")]
fn write_indent(dst: &mut impl Write, depth: usize, hard_tab: bool) -> fmt::Result {
    let indentor = if hard_tab { "\t" } else { "\x20\x20\x20\x20" };
    (0..depth).try_for_each(|_| dst.write_str(indentor))
}

fn write_bool(dst: &mut impl Write, b: bool) -> fmt::Result {
    match b {
        true => dst.write_str("true"),
        false => dst.write_str("false"),
    }
}

struct EscapedWriter<W: Write>(W);

impl<W: Write> Write for EscapedWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.chars().try_for_each(|ch| self.write_char(ch))
    }

    fn write_char(&mut self, ch: char) -> fmt::Result {
        write_escaped_char::<true>(&mut self.0, ch)
    }
}

fn write_quoted_char(dst: &mut impl Write, ch: char) -> fmt::Result {
    dst.write_str(r#"'"#)?;
    write_escaped_char::<false>(dst, ch)?;
    dst.write_str(r#"'"#)
}

fn write_quoted_string(dst: &mut impl Write, s: &str) -> fmt::Result {
    dst.write_str(r#"""#)?;
    s.chars().try_for_each(|ch| write_escaped_char::<true>(dst, ch))?;
    dst.write_str(r#"""#)
}

fn write_quoted_byte_string(dst: &mut impl Write, bytes: &[u8]) -> fmt::Result {
    dst.write_str(r#"b""#)?;
    bytes
        .iter()
        .try_for_each(|&byte| write_escaped_byte::<true>(dst, byte))?;
    dst.write_str(r#"""#)
}

#[inline(always)]
fn write_escaped_char<const IN_STR: bool>(dst: &mut impl Write, ch: char) -> fmt::Result {
    match ch {
        '\0' => dst.write_str(r#"\0"#),
        '\n' => dst.write_str(r#"\n"#),
        '\t' => dst.write_str(r#"\t"#),
        '\r' => dst.write_str(r#"\r"#),
        '\"' if IN_STR => dst.write_str(r#"\""#),
        '\'' if !IN_STR => dst.write_str(r#"\'"#),
        ch if ch.is_ascii_control() => {
            dst.write_str(r#"\x"#)?;
            write_u8_fmt_02_hex(dst, ch as u8)
        }
        ch => dst.write_char(ch),
    }
}

#[inline(always)]
fn write_escaped_byte<const IN_STR: bool>(dst: &mut impl Write, byte: u8) -> fmt::Result {
    match byte {
        b'\0' => dst.write_str(r#"\0"#),
        b'\n' => dst.write_str(r#"\n"#),
        b'\t' => dst.write_str(r#"\t"#),
        b'\r' => dst.write_str(r#"\r"#),
        b'\"' if IN_STR => dst.write_str(r#"\""#),
        b'\'' if !IN_STR => dst.write_str(r#"\'"#),
        byte if !byte.is_ascii_control() => dst.write_char(byte.into()),
        byte => {
            dst.write_str(r#"\x"#)?;
            write_u8_fmt_02_hex(dst, byte)
        }
    }
}

#[inline(always)]
fn write_u8_fmt_02_hex(dst: &mut impl Write, byte: u8) -> fmt::Result {
    use lexical_write_integer::{NumberFormatBuilder, Options, ToLexicalWithOptions};

    let mut buf = [b'0'; 2];
    byte.to_lexical_with_options::<{ NumberFormatBuilder::hexadecimal() }>(
        &mut buf[(byte < 0x10) as usize..],
        &Options::new(),
    );

    dst.write_str(unsafe { core::str::from_utf8_unchecked(&buf) })
}

//------------------------------------------------------------------------------

fn write_i64(dst: &mut impl Write, n: i64) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i64::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { core::str::from_utf8_unchecked(digits) })
}

fn write_i128(dst: &mut impl Write, n: i128) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i128::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { core::str::from_utf8_unchecked(digits) })
}

fn write_u64(dst: &mut impl Write, n: u64) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i64::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { core::str::from_utf8_unchecked(digits) })
}

fn write_u128(dst: &mut impl Write, n: u128) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i128::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { core::str::from_utf8_unchecked(digits) })
}

fn write_f32(dst: &mut impl Write, n: f32) -> fmt::Result {
    dst.write_str(zmij::Buffer::new().format(n))
}

fn write_f64(dst: &mut impl Write, n: f64) -> fmt::Result {
    dst.write_str(zmij::Buffer::new().format(n))
}

fn write_number_suffix(dst: &mut impl Write, suff: NumberSuffix, flags: Flags) -> fmt::Result {
    macro_rules! cond_write {
        ($dst:ident, $cond:expr, $suff:literal) => {{
            if $cond {
                $dst.write_str($suff)?;
            }
            Ok(())
        }};
    }
    let int_suff = flags.write_integer_suffix();
    let float_suff = flags.write_float_suffix();
    match suff {
        NumberSuffix::Int128 => dst.write_str("i128"),
        NumberSuffix::UInt128 => dst.write_str("u128"),
        NumberSuffix::Int8 => cond_write!(dst, int_suff, "i8"),
        NumberSuffix::Int16 => cond_write!(dst, int_suff, "i16"),
        NumberSuffix::Int32 => cond_write!(dst, int_suff, "i32"),
        NumberSuffix::Int64 => cond_write!(dst, int_suff, "i64"),
        NumberSuffix::UInt8 => cond_write!(dst, int_suff, "u8"),
        NumberSuffix::UInt16 => cond_write!(dst, int_suff, "u16"),
        NumberSuffix::UInt32 => cond_write!(dst, int_suff, "u32"),
        NumberSuffix::UInt64 => cond_write!(dst, int_suff, "u64"),
        NumberSuffix::Float32 => cond_write!(dst, float_suff, "f32"),
        NumberSuffix::Float64 => cond_write!(dst, float_suff, "f64"),
    }
}

fn write_number(dst: &mut impl Write, num: &Number, flags: Flags) -> fmt::Result {
    match *num {
        Number::Int8(v) => write_i64(dst, v.into()),
        Number::Int16(v) => write_i64(dst, v.into()),
        Number::Int32(v) => write_i64(dst, v.into()),
        Number::Int64(v) => write_i64(dst, v),
        Number::IntNoSuffix(v) => write_i64(dst, v),

        Number::UInt8(v) => write_u64(dst, v.into()),
        Number::UInt16(v) => write_u64(dst, v.into()),
        Number::UInt32(v) => write_u64(dst, v.into()),
        Number::UInt64(v) => write_u64(dst, v),
        Number::UIntNoSuffix(v) => write_u64(dst, v),

        Number::Float32(Float32(v)) => write_f32(dst, v),
        Number::Float64(Float64(v)) => write_f64(dst, v),
        Number::FloatNoSuffix(Float64(v)) => write_f64(dst, v),

        Number::Int128 { lo, hi } => write_i128(dst, (hi as i128) << 64 | lo as i128),
        Number::UInt128 { lo, hi } => write_u128(dst, (hi as u128) << 64 | lo as u128),
    }?;
    if let Some(suff) = num.suffix() {
        write_number_suffix(dst, suff, flags)?;
    }
    Ok(())
}

fn write_scalar(dst: &mut impl Write, scalar: &Scalar, flags: Flags) -> fmt::Result {
    match scalar {
        Scalar::Char(ch) => write_quoted_char(dst, *ch),
        Scalar::Number(num) => write_number(dst, num, flags),
    }
}

//------------------------------------------------------------------------------

fn write_struct_name(dst: &mut impl Write, name: Option<&Ident>, flags: Flags) -> fmt::Result {
    if !flags.omit_struct_name() && name.is_some() {
        dst.write_str(name.unwrap())
    } else {
        dst.write_str("_")
    }
}

fn write_newtype_name(dst: &mut impl Write, name: Option<&Ident>, flags: Flags) -> fmt::Result {
    if !flags.implicit_newtype() {
        if !flags.omit_newtype_name() && name.is_some() {
            dst.write_str("~")?;
            dst.write_str(name.unwrap())?;
            dst.write_str(" ")?;
        } else {
            dst.write_str("!")?;
        }
    }
    Ok(())
}

fn write_variant_name(dst: &mut impl Write, name: Option<&Ident>, variant: &Ident, flags: Flags) -> fmt::Result {
    if !flags.implicit_variant() {
        if !flags.omit_enum_name() && name.is_some() {
            dst.write_str(name.unwrap())?;
            dst.write_str("::")?;
        } else {
            dst.write_str(".")?;
        }
    }
    dst.write_str(variant)
}
