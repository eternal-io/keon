use crate::{value::*, PrivateMethod};
use core::{
    fmt::{self, Write},
    ops::{Deref, DerefMut},
};

#[cfg(feature = "alloc")]
use alloc::collections::VecDeque;

mod ser_concr;
mod ser_value;

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

//------------------------------------------------------------------------------

#[expect(private_interfaces, reason = "Sealed")]
pub trait Serialize {
    #[doc(hidden)]
    fn serialize_with<Impl: SerializerImpl>(&self, ser: &mut Serializer<Impl>, _: PrivateMethod) -> fmt::Result;
}

#[expect(private_bounds, reason = "Sealed")]
pub trait SerializerImpl: SerializerImplDetail {}

trait SerializerImplDetail {
    #[cfg(feature = "alloc")]
    fn push_stringified(&mut self, stringified: String) -> fmt::Result;

    fn push_bool(&mut self, b: bool) -> fmt::Result;
    fn push_char(&mut self, ch: char) -> fmt::Result;
    fn push_number(&mut self, num: &Number2) -> fmt::Result;
    fn push_str(&mut self, s: &str) -> fmt::Result;
    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result;

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
    fn push_map_struct_begin(&mut self, name: Option<&Ident>) -> fmt::Result;
    fn push_map_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result;
    fn push_map_like_end(&mut self) -> fmt::Result;

    fn push_newtype_begin(&mut self, name: Option<&Ident>) -> fmt::Result;
    fn push_newtype_end(&mut self) -> fmt::Result;

    fn push_identifier(&mut self, field: &Ident) -> fmt::Result;
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
        Self::with_flags(dst, Flags::COMPACT_OMIT_NAMES)
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
        const HARD_TAB              = 1 << 0;

        const WRITE_INTEGER_SUFFIX  = 1 << 1;
        const WRITE_FLOAT_SUFFIX    = 1 << 2;

        const MAX_WIDTH_PLUS_8      = 1 << 3;
        const MAX_WIDTH_PLUS_16     = 1 << 4;
        const MAX_WIDTH_PLUS_32     = 1 << 5;
        const MAX_WIDTH_PLUS_64     = 1 << 6;

        const OMIT_STRUCT_NAME      = 1 << 7;
        const OMIT_NEWTYPE_NAME     = 1 << 8;
        const IMPLICIT_NEWTYPE      = 1 << 9;
        const OMIT_ENUM_NAME        = 1 << 10;
        const IMPLICIT_VARIANT      = 1 << 11;

        const COMPACTIZE_MAP_KEY    = 1 << 12;
        const EXPAND_ROOT_STRUCTURE = 1 << 13;

        const WRITE_NUMBER_SUFFIX
            = Self::WRITE_INTEGER_SUFFIX.bits()
            | Self::WRITE_FLOAT_SUFFIX.bits();

        const MAX_WIDTH_EQUAL_120
            = Self::MAX_WIDTH_PLUS_8.bits()
            | Self::MAX_WIDTH_PLUS_16.bits()
            | Self::MAX_WIDTH_PLUS_32.bits()
            | Self::MAX_WIDTH_PLUS_64.bits();

        const OMIT_STRUCTURE_NAMES
            = Self::OMIT_STRUCT_NAME.bits()
            | Self::OMIT_NEWTYPE_NAME.bits()
            | Self::OMIT_ENUM_NAME.bits();

        const IMPLICIT_STRUCTURES
            = Self::IMPLICIT_NEWTYPE.bits()
            | Self::IMPLICIT_VARIANT.bits();

        const EXPANDED
            = Self::EXPAND_ROOT_STRUCTURE.bits();

        const DEFAULT
            = Self::MAX_WIDTH_EQUAL_120.bits()
            | Self::COMPACTIZE_MAP_KEY.bits()
            | Self::EXPAND_ROOT_STRUCTURE.bits();

        const COMPACT_OMIT_NAMES
            = Self::HARD_TAB.bits()
            | Self::MAX_WIDTH_EQUAL_120.bits()
            | Self::OMIT_STRUCTURE_NAMES.bits()
            | Self::COMPACTIZE_MAP_KEY.bits();

        const COMPACT_IMPLICIT_ALL
            = Self::HARD_TAB.bits()
            | Self::MAX_WIDTH_EQUAL_120.bits()
            | Self::OMIT_STRUCTURE_NAMES.bits()
            | Self::IMPLICIT_STRUCTURES.bits()
            | Self::COMPACTIZE_MAP_KEY.bits();
    }
}

impl Flags {
    pub fn hard_tab(&self) -> bool {
        self.contains(Self::HARD_TAB)
    }
    pub fn max_width(&self) -> usize {
        self.intersection(Self::MAX_WIDTH_EQUAL_120).bits() as usize
    }

    pub fn write_integer_suffix(&self) -> bool {
        self.contains(Self::WRITE_INTEGER_SUFFIX)
    }
    pub fn write_float_suffix(&self) -> bool {
        self.contains(Self::WRITE_FLOAT_SUFFIX)
    }

    pub fn omit_struct_name(&self) -> bool {
        self.contains(Self::OMIT_STRUCT_NAME)
    }
    pub fn omit_newtype_name(&self) -> bool {
        self.contains(Self::OMIT_NEWTYPE_NAME)
    }
    pub fn implicit_newtype(&self) -> bool {
        self.contains(Self::IMPLICIT_NEWTYPE)
    }
    pub fn omit_enum_name(&self) -> bool {
        self.contains(Self::OMIT_ENUM_NAME)
    }
    pub fn implicit_variant(&self) -> bool {
        self.contains(Self::IMPLICIT_VARIANT)
    }

    pub fn compactize_map_key(&self) -> bool {
        self.contains(Self::COMPACTIZE_MAP_KEY)
    }
    pub fn expand_root_structure(&self) -> bool {
        self.contains(Self::EXPAND_ROOT_STRUCTURE)
    }
}

//==================================================================================================

pub struct FastImpl<W> {
    dst: W,
    flags: Flags,
    range_intercept: Option<RangeType>,
    range_start: Option<Option<Scalar>>,
    range_end: Option<Option<Scalar>>,
}

enum RangeType {
    RangeTo,
    RangeToInclusive,
    RangeFrom,
    Range,
    RangeInclusive,
}

impl<W> FastImpl<W> {
    fn new(dst: W, flags: Flags) -> Self {
        Self {
            dst,
            flags,
            range_intercept: None,
            range_start: None,
            range_end: None,
        }
    }
}

impl<W: Write> FastImpl<W> {
    #[inline(always)]
    fn clear_range_intercept(&mut self) -> fmt::Result {
        if self.range_intercept.is_some() {
            self.clear_range_intercept_cold()?;
        }
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn clear_range_intercept_cold(&mut self) -> fmt::Result {
        if let Some(typ) = self.range_intercept.take() {
            self.dst.write_str(match typ {
                RangeType::RangeTo => "RangeTo",
                RangeType::RangeToInclusive => "RangeToInclusive",
                RangeType::RangeFrom => "RangeFrom",
                RangeType::Range => "Range",
                RangeType::RangeInclusive => "RangeInclusive",
            })?;
            self.dst.write_str("{")?;
        }
        if let Some(start) = self.range_start.take() {
            self.dst.write_str("start")?;
            if let Some(scalar) = start {
                self.dst.write_str(":")?;
                write_scalar(&mut self.dst, &scalar, self.flags)?;
                self.dst.write_str(",")?;
            }
        }
        if let Some(end) = self.range_end.take() {
            self.dst.write_str("end")?;
            if let Some(scalar) = end {
                self.dst.write_str(":")?;
                write_scalar(&mut self.dst, &scalar, self.flags)?;
                self.dst.write_str(",")?;
            }
        }
        Ok(())
    }

    fn intercept_range_field(&mut self, field: &Ident) -> Result<bool, fmt::Error> {
        let Some(ref typ) = self.range_intercept else {
            return Ok(false);
        };

        let field = field.as_str();
        #[rustfmt::skip]
        if field == "start"
            && self.range_start.is_none()
            && matches!(typ, RangeType::Range | RangeType::RangeInclusive | RangeType::RangeFrom)
        {
            self.range_start = Some(None);
        } else if field == "end"
            && self.range_end.is_none()
            && matches!(typ, RangeType::Range | RangeType::RangeInclusive | RangeType::RangeTo | RangeType::RangeToInclusive)
        {
            self.range_end = Some(None);
        } else {
            self.clear_range_intercept_cold()?;
            return Ok(false);
        };

        Ok(true)
    }

    fn intercept_range_bound<T, F>(&mut self, scalar: T, callback: F) -> fmt::Result
    where
        T: Into<Scalar>,
        F: FnOnce(&mut Self, T) -> fmt::Result,
    {
        if self.range_intercept.is_some() {
            Ok(self.intercept_range_bound_cold(scalar))
        } else {
            callback(self, scalar)
        }
    }

    fn intercept_range_bound_cold<T>(&mut self, scalar: T)
    where
        T: Into<Scalar>,
    {
        if let Some(start @ None) = self.range_start.as_mut() {
            *start = Some(scalar.into());
        } else if let Some(end @ None) = self.range_start.as_mut() {
            *end = Some(scalar.into());
        } else {
            unreachable!()
        }
    }
}

impl<W: Write> SerializerImpl for FastImpl<W> {}

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

impl<W: Write> SerializerImplDetail for FastImpl<W> {
    fn push_stringified(&mut self, _stringified: String) -> fmt::Result {
        panic!("FastImpl does not rely on alloc")
    }

    fn push_bool(&mut self, b: bool) -> fmt::Result {
        self.clear_range_intercept()?;
        write_bool(&mut self.dst, b)
    }
    fn push_char(&mut self, ch: char) -> fmt::Result {
        self.intercept_range_bound(ch, |ser, ch| write_quoted_char(&mut ser.dst, ch))
    }
    fn push_number(&mut self, num: &Number2) -> fmt::Result {
        self.intercept_range_bound(num, |ser, num| write_number(&mut ser.dst, num, ser.flags))
    }
    fn push_str(&mut self, s: &str) -> fmt::Result {
        self.clear_range_intercept()?;
        write_quoted_string(&mut self.dst, s)
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
        debug_assert!(self.range_intercept.is_none());

        Ok(())
    }

    fn push_array_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("[")
    }
    fn push_array_end(&mut self) -> fmt::Result {
        debug_assert!(self.range_intercept.is_none());

        self.dst.write_str("]")
    }

    fn push_unit(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("()")
    }
    fn push_unit_struct(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        write_struct_name(&mut self.dst, name, self.flags)
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
        debug_assert!(self.range_intercept.is_none());

        self.dst.write_str(")")
    }

    fn push_map_begin(&mut self) -> fmt::Result {
        self.clear_range_intercept()?;
        self.dst.write_str("{")
    }
    fn push_map_struct_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        if let Some(name) = name {
            'range_intercept: {
                self.range_intercept = Some(match name.as_str() {
                    "RangeTo" => RangeType::RangeTo,
                    "RangeToInclusive" => RangeType::RangeToInclusive,
                    "RangeFrom" => RangeType::RangeFrom,
                    "Range" => RangeType::Range,
                    "RangeInclusive" => RangeType::RangeInclusive,
                    _ => break 'range_intercept,
                });
                return Ok(());
            }
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
        if let Some(typ) = self.range_intercept.take() {
            'range_intercept: {
                match typ {
                    RangeType::RangeTo => {
                        if let (None, Some(Some(end))) = (self.range_start, self.range_end) {
                            self.dst.write_str("..")?;
                            write_scalar(&mut self.dst, &end, self.flags)?;
                        } else {
                            break 'range_intercept;
                        }
                    }
                    RangeType::RangeToInclusive => {
                        if let (None, Some(Some(end))) = (self.range_start, self.range_end) {
                            self.dst.write_str("..=")?;
                            write_scalar(&mut self.dst, &end, self.flags)?;
                        } else {
                            break 'range_intercept;
                        }
                    }
                    RangeType::RangeFrom => {
                        if let (Some(Some(start)), None) = (self.range_start, self.range_end) {
                            write_scalar(&mut self.dst, &start, self.flags)?;
                            self.dst.write_str("..")?;
                        } else {
                            break 'range_intercept;
                        }
                    }
                    RangeType::Range => {
                        if let (Some(Some(start)), Some(Some(end))) = (self.range_start, self.range_end) {
                            write_scalar(&mut self.dst, &start, self.flags)?;
                            self.dst.write_str("..")?;
                            write_scalar(&mut self.dst, &end, self.flags)?;
                        } else {
                            break 'range_intercept;
                        }
                    }
                    RangeType::RangeInclusive => {
                        if let (Some(Some(start)), Some(Some(end))) = (self.range_start, self.range_end) {
                            write_scalar(&mut self.dst, &start, self.flags)?;
                            self.dst.write_str("..=")?;
                            write_scalar(&mut self.dst, &end, self.flags)?;
                        } else {
                            break 'range_intercept;
                        }
                    }
                };
                self.range_start = None;
                self.range_end = None;
                return Ok(());
            }
            self.clear_range_intercept_cold()?;
        }
        self.dst.write_str("}")
    }

    fn push_newtype_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        self.clear_range_intercept()?;
        write_newtype_name(&mut self.dst, name, self.flags)
    }
    fn push_newtype_end(&mut self) -> fmt::Result {
        debug_assert!(self.range_intercept.is_none());

        Ok(())
    }

    fn push_identifier(&mut self, field: &Ident) -> fmt::Result {
        if self.intercept_range_field(field)? {
            return Ok(());
        }
        self.dst.write_str(field)
    }
    fn push_fat_arrow(&mut self) -> fmt::Result {
        debug_assert!(self.range_intercept.is_none());

        self.dst.write_str("=>")
    }
    fn push_colon(&mut self) -> fmt::Result {
        if self.range_intercept.is_some() {
            return Ok(());
        }
        self.dst.write_str(":")
    }
    fn push_comma(&mut self) -> fmt::Result {
        if self.range_intercept.is_some() {
            return Ok(());
        }
        self.dst.write_str(",")
    }

    fn semicolon(&mut self) -> fmt::Result {
        debug_assert!(self.range_intercept.is_none());

        self.dst.write_str(";")
    }
}

//------------------------------------------------------------------------------

#[cfg(feature = "alloc")]
pub struct PrettyImpl<W> {
    dst: W,
    flags: Flags,
    compounds_stack: Vec<Compound>,
    inline_entries: VecDeque<String>,
}

#[cfg(feature = "alloc")]
impl<W> PrettyImpl<W> {
    fn new(dst: W, flags: Flags) -> Self {
        Self {
            dst,
            flags,
            compounds_stack: Vec::new(),
            inline_entries: VecDeque::new(),
        }
    }
}

#[cfg(feature = "alloc")]
impl<W> Drop for PrettyImpl<W> {
    fn drop(&mut self) {
        debug_assert!(self.compounds_stack.is_empty());
        debug_assert!(self.inline_entries.is_empty());
    }
}

#[cfg(feature = "alloc")]
enum Compound {
    Compact {
        kind: CompoundKind,
        force_compact: bool,
        inline_entries_index: usize,
    },
    Expanded {
        kind: CompoundKind,
    },
}

#[cfg(feature = "alloc")]
impl Compound {
    fn kind(&self) -> &CompoundKind {
        match self {
            Compound::Compact { kind, .. } | Compound::Expanded { kind } => kind,
        }
    }
}

#[cfg(feature = "alloc")]
enum CompoundKind {
    Maybe,
    Array,
    TupleLike(Option<String>),
    MapLikeLhs(Option<String>),
    MapLikeRhs(Option<String>),
}

#[cfg(feature = "alloc")]
impl CompoundKind {
    fn new_line_child(&self) -> bool {
        !matches!(self, CompoundKind::Maybe | CompoundKind::MapLikeRhs(_))
    }

    fn is_collection(&self) -> bool {
        !matches!(self, CompoundKind::Maybe)
    }

    fn is_map_like(&self) -> bool {
        matches!(self, CompoundKind::MapLikeLhs(_) | CompoundKind::MapLikeRhs(_))
    }

    fn is_map_like_lhs(&self) -> bool {
        matches!(self, CompoundKind::MapLikeLhs(_))
    }

    fn take(&mut self) -> Self {
        match self {
            CompoundKind::Maybe => CompoundKind::Maybe,
            CompoundKind::Array => CompoundKind::Array,
            CompoundKind::TupleLike(head) => CompoundKind::TupleLike(head.take()),
            CompoundKind::MapLikeLhs(head) => CompoundKind::MapLikeLhs(head.take()),
            CompoundKind::MapLikeRhs(head) => CompoundKind::MapLikeRhs(head.take()),
        }
    }

    fn write_indicator_to(&self, dst: &mut impl Write) -> fmt::Result {
        match self {
            CompoundKind::Maybe => dst.write_str("?"),
            CompoundKind::Array => dst.write_str("["),
            CompoundKind::TupleLike(head) => {
                if let Some(head) = head {
                    dst.write_str(head)?;
                }
                dst.write_str("(")
            }
            CompoundKind::MapLikeLhs(head) | CompoundKind::MapLikeRhs(head) => {
                if let Some(head) = head {
                    dst.write_str(head)?;
                    dst.write_str(" ")?;
                }
                dst.write_str("{")
            }
        }
    }

    fn write_terminator_to(&self, dst: &mut impl Write) -> fmt::Result {
        match self {
            CompoundKind::Maybe => Ok(()),
            CompoundKind::Array => dst.write_str("]"),
            CompoundKind::TupleLike(_) => dst.write_str(")"),
            CompoundKind::MapLikeLhs(_) | CompoundKind::MapLikeRhs(_) => dst.write_str("}"),
        }
    }
}

#[cfg(feature = "alloc")]
impl<W: Write> SerializerImpl for PrettyImpl<W> {}

macro_rules! push_concr_number_pretty {
    ($method:ident, $ty:ty) => {
        fn $method(&mut self, n: $ty) -> fmt::Result {
            todo!()
        }
    };
}

#[cfg(feature = "alloc")]
impl<W: Write> SerializerImplDetail for PrettyImpl<W> {
    /*
    #[doc(hidden)]
    fn push(&mut self, token: Token) -> fmt::Result {
        let dst = &mut self.dst;
        let cfg = &self.cfg;
        let direct_write_indent =
            |dst: &mut W, depth: usize| -> fmt::Result { (0..depth).try_for_each(|_| cfg.indentor.write_to(dst)) };

        let direct_write_indicator =
            |dst: &mut W, depth: usize, new_line_child: bool, kind: &CompoundKind| -> fmt::Result {
                match new_line_child {
                    true => direct_write_indent(dst, depth)?,
                    false => dst.write_str(" ")?,
                }
                kind.write_indicator_to(dst)?;
                match kind.is_collection() {
                    true => dst.write_str("\n"),
                    false => Ok(()),
                }
            };

        let direct_write_entry = |dst: &mut W, depth: usize, kind: &mut CompoundKind, entry: &str| -> fmt::Result {
            match kind.new_line_child() {
                true => direct_write_indent(dst, depth)?,
                false => dst.write_str(" ")?,
            }
            dst.write_str(entry)?;
            match kind {
                CompoundKind::MapLikeLhs(head @ None) => {
                    *kind = CompoundKind::MapLikeRhs(head.take());
                    dst.write_str(" =>")
                }
                CompoundKind::MapLikeLhs(head @ Some(_)) => {
                    *kind = CompoundKind::MapLikeRhs(head.take());
                    dst.write_str(":")
                }
                CompoundKind::MapLikeRhs(head) => {
                    *kind = CompoundKind::MapLikeLhs(head.take());
                    dst.write_str(",\n")
                }
                kind => match kind.is_collection() {
                    true => dst.write_str(",\n"),
                    false => Ok(()),
                },
            }
        };

        let break_and_flush =
            |dst: &mut W, compounds_stack: &mut Vec<Compound>, inline_entries: &mut VecDeque<String>| -> fmt::Result {
                /* The force-compact check is performed externally; if it is true, this closure is not called. */
                let compounds_count = compounds_stack.len();
                let mut new_line_child;
                for i in 0..compounds_count {
                    if let Compound::Expanded { .. } = compounds_stack[i] {
                        continue;
                    }

                    new_line_child = i
                        .checked_sub(1)
                        .map(|i| compounds_stack[i].kind().new_line_child())
                        .unwrap_or(true);

                    let range_end = match compounds_stack.get(i + 1) {
                        Some(Compound::Compact {
                            inline_entries_index, ..
                        }) => *inline_entries_index,
                        _ => compounds_count,
                    };
                    let Compound::Compact {
                        ref mut kind,
                        inline_entries_index: range_start,
                        ..
                    } = compounds_stack[i]
                    else {
                        unreachable!()
                    };

                    direct_write_indicator(dst, i, new_line_child, kind)?;
                    inline_entries
                        .drain(..range_end - range_start)
                        .try_for_each(|entry| direct_write_entry(dst, i + 1, kind, &entry))?;

                    compounds_stack[i] = Compound::Expanded { kind: kind.take() };
                }
                Ok(())
            };

        let literal_to_string = |literal: Literal<'_>| -> Result<String, fmt::Error> {
            let mut stringified = String::with_capacity(256);
            write_literal(&mut stringified, literal, cfg.numeric_suffix)?;
            Ok(stringified)
        };

        let nominal_path_to_string = |kind: NominalKind, path: NominalPathRef| -> Result<String, fmt::Error> {
            let mut stringified = String::with_capacity(64);
            write_nominal_path(&mut stringified, kind, path, cfg.nominal_path_style)?;
            Ok(stringified)
        };

        match token {
            Token::Stringified(_) | Token::Literal(_) | Token::Ident(_) | Token::Unit | Token::UnitStruct { .. } => {
                let entry = match token {
                    Token::Stringified(entry) => entry,
                    Token::Literal(literal) => literal_to_string(literal)?,
                    Token::Ident(ident) => ident.to_string(),
                    Token::Unit => "()".to_string(),
                    Token::UnitStruct { kind, path } => nominal_path_to_string(kind, path)?,
                    _ => unreachable!(),
                };
                match self.compounds_stack.last() {
                    Some(comp) => match comp {
                        Compound::Compact {
                            kind,
                            force_compact,
                            inline_entries_index,
                        } => {
                            self.inline_entries.push_back(entry);
                            if !*force_compact
                                && if kind.is_map_like() {
                                    /* conditionally expand `{}` */
                                    self.inline_entries.len() - *inline_entries_index
                                        > self.cfg.map_like_inline_entries as usize
                                } else {
                                    /* unconditionally compact `?`, `[]` and `()` while pushing stringified */
                                    false
                                }
                            {
                                break_and_flush(dst, &mut self.compounds_stack, &mut self.inline_entries)?;
                            }
                        }
                        Compound::Expanded { .. } => {
                            let depth = self.compounds_stack.len();
                            let Compound::Expanded { ref mut kind } = self.compounds_stack.last_mut().unwrap() else {
                                unreachable!()
                            };
                            direct_write_entry(dst, depth, kind, &entry)?;
                        }
                    },
                    None => {
                        dst.write_str(&entry)?;
                        dst.write_str(";\n")?;
                    }
                }
            }

            Token::MaybeEnd | Token::ArrayEnd | Token::TupleEnd | Token::MapLikeEnd => {
                let debug_assert_matches = |token: &Token<'_>, kind: &CompoundKind| match kind {
                    CompoundKind::Maybe => debug_assert!(matches!(token, Token::MaybeEnd)),
                    CompoundKind::Array => debug_assert!(matches!(token, Token::ArrayEnd)),
                    CompoundKind::TupleLike(_) => debug_assert!(matches!(token, Token::TupleEnd)),
                    CompoundKind::MapLikeLhs(_) | CompoundKind::MapLikeRhs(_) => {
                        debug_assert!(matches!(token, Token::MapLikeEnd))
                    }
                };
                match self.compounds_stack.pop().unwrap() {
                    Compound::Compact {
                        kind,
                        inline_entries_index,
                        ..
                    } => {
                        debug_assert_matches(&token, &kind);

                        let entries_count = self.inline_entries.len() - inline_entries_index;
                        let mut entries = self.inline_entries.drain(inline_entries_index..);
                        let mut stringified = String::with_capacity(256);

                        kind.write_indicator_to(&mut stringified)?;
                        if (kind.is_map_like() || !kind.is_collection()) && entries_count > 0 {
                            dst.write_str(" ")?;
                        }
                        for _ in 0..entries_count.saturating_sub(1) {
                            dst.write_str(&entries.next().unwrap())?;
                            dst.write_str(", ")?;
                        }
                        if let Some(entry) = entries.next() {
                            dst.write_str(&entry)?;
                        }
                        if kind.is_map_like() && entries_count > 0 {
                            dst.write_str(" ")?;
                        }
                        kind.write_terminator_to(&mut stringified)?;
                        drop(entries);

                        self.push(Token::Stringified(stringified))?;
                    }
                    Compound::Expanded { kind } => {
                        debug_assert_matches(&token, &kind);

                        direct_write_indent(dst, self.compounds_stack.len())?;

                        kind.write_terminator_to(dst)?;

                        match self.compounds_stack.last() {
                            Some(comp) => match comp.kind().is_collection() {
                                true => dst.write_str(",\n")?,
                                false => (),
                            },
                            None => dst.write_str(";\n")?,
                        }
                    }
                }
            }

            Token::FatArrow | Token::Colon | Token::Comma => (),

            token => {
                let kind = match token {
                    Token::Maybe => CompoundKind::Maybe,
                    Token::Array => CompoundKind::Array,
                    Token::Tuple => CompoundKind::TupleLike(None),
                    Token::TupleStruct { kind, path } => {
                        CompoundKind::TupleLike(Some(nominal_path_to_string(kind, path)?))
                    }
                    Token::Map => CompoundKind::MapLikeLhs(None),
                    Token::MapStruct { kind, path } => {
                        CompoundKind::MapLikeLhs(Some(nominal_path_to_string(kind, path)?))
                    }
                    _ => unreachable!(),
                };
                let force_compact = match self.compounds_stack.last() {
                    Some(comp) => match comp {
                        Compound::Compact { force_compact, .. } => *force_compact,
                        /* unconditionally compact `=>` left-hand side */
                        Compound::Expanded { kind } => kind.is_map_like_lhs(),
                    },
                    None => false,
                };
                if !force_compact {
                    /* conditionally expand parent while pushing compound */
                    break_and_flush(dst, &mut self.compounds_stack, &mut self.inline_entries)?;
                }
                self.compounds_stack.push(Compound::Compact {
                    kind,
                    force_compact,
                    inline_entries_index: self.inline_entries.len(),
                });
            }
        }

        Ok(())
    }
    */

    fn push_stringified(&mut self, stringified: String) -> fmt::Result {
        todo!()
    }

    fn push_bool(&mut self, b: bool) -> fmt::Result {
        todo!()
    }
    fn push_char(&mut self, ch: char) -> fmt::Result {
        todo!()
    }
    fn push_number(&mut self, num: &Number2) -> fmt::Result {
        todo!()
    }
    fn push_str(&mut self, s: &str) -> fmt::Result {
        todo!()
    }
    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
        todo!()
    }

    push_concr_number_pretty!(push_i8, i8);
    push_concr_number_pretty!(push_i16, i16);
    push_concr_number_pretty!(push_i32, i32);
    push_concr_number_pretty!(push_i64, i64);
    push_concr_number_pretty!(push_i128, i128);
    push_concr_number_pretty!(push_u8, u8);
    push_concr_number_pretty!(push_u16, u16);
    push_concr_number_pretty!(push_u32, u32);
    push_concr_number_pretty!(push_u64, u64);
    push_concr_number_pretty!(push_u128, u128);
    push_concr_number_pretty!(push_f32, f32);
    push_concr_number_pretty!(push_f64, f64);

    fn push_range_full(&mut self) -> fmt::Result {
        let stringified = String::from("..");
        self.push_stringified(stringified)
    }
    fn push_range_to(&mut self, end: &Scalar) -> fmt::Result {
        let mut stringified = String::from("..");
        write_scalar(&mut stringified, end, self.flags)?;
        self.push_stringified(stringified)
    }
    fn push_range_to_inclusive(&mut self, end: &Scalar) -> fmt::Result {
        let mut stringified = String::from("..=");
        write_scalar(&mut stringified, end, self.flags)?;
        self.push_stringified(stringified)
    }
    fn push_range_from(&mut self, start: &Scalar) -> fmt::Result {
        let mut stringified = String::new();
        write_scalar(&mut stringified, start, self.flags)?;
        stringified.push_str("..");
        self.push_stringified(stringified)
    }
    fn push_range(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        let mut stringified = String::new();
        write_scalar(&mut stringified, start, self.flags)?;
        stringified.push_str("..");
        write_scalar(&mut stringified, end, self.flags)?;
        self.push_stringified(stringified)
    }
    fn push_range_inclusive(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        let mut stringified = String::new();
        write_scalar(&mut stringified, start, self.flags)?;
        stringified.push_str("..=");
        write_scalar(&mut stringified, end, self.flags)?;
        self.push_stringified(stringified)
    }

    fn push_maybe_begin(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_maybe_end(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_array_begin(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_array_end(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_unit(&mut self) -> fmt::Result {
        todo!()
    }
    fn push_unit_struct(&mut self, name: Option<&Ident>) -> fmt::Result {
        todo!()
    }
    fn push_unit_variant(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        todo!()
    }

    fn push_tuple_begin(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_tuple_struct_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        todo!()
    }

    fn push_tuple_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        todo!()
    }

    fn push_tuple_like_end(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_map_begin(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_map_struct_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        todo!()
    }

    fn push_map_variant_begin(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result {
        todo!()
    }

    fn push_map_like_end(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_newtype_begin(&mut self, name: Option<&Ident>) -> fmt::Result {
        todo!()
    }

    fn push_newtype_end(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_identifier(&mut self, field: &Ident) -> fmt::Result {
        todo!()
    }

    fn push_fat_arrow(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_colon(&mut self) -> fmt::Result {
        todo!()
    }

    fn push_comma(&mut self) -> fmt::Result {
        todo!()
    }

    fn semicolon(&mut self) -> fmt::Result {
        todo!()
    }
}

//==================================================================================================

fn write_bool(dst: &mut impl Write, b: bool) -> fmt::Result {
    match b {
        true => dst.write_str("true"),
        false => dst.write_str("false"),
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
        .into_iter()
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

    dst.write_str(unsafe { ::core::str::from_utf8_unchecked(&buf) })
}

//------------------------------------------------------------------------------

fn write_i64(dst: &mut impl Write, n: i64) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i64::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { ::core::str::from_utf8_unchecked(digits) })
}

fn write_i128(dst: &mut impl Write, n: i128) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i128::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { ::core::str::from_utf8_unchecked(digits) })
}

fn write_u64(dst: &mut impl Write, n: u64) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i64::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { ::core::str::from_utf8_unchecked(digits) })
}

fn write_u128(dst: &mut impl Write, n: u128) -> fmt::Result {
    use lexical_write_integer::{FormattedSize, ToLexical};
    let mut buf = [0; i128::FORMATTED_SIZE_DECIMAL];
    let digits = n.to_lexical(&mut buf);
    dst.write_str(unsafe { ::core::str::from_utf8_unchecked(digits) })
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

fn write_number(dst: &mut impl Write, num: &Number2, flags: Flags) -> fmt::Result {
    match *num {
        Number2::Int8(v) => write_i64(dst, v.into()),
        Number2::Int16(v) => write_i64(dst, v.into()),
        Number2::Int32(v) => write_i64(dst, v.into()),
        Number2::Int64(v) => write_i64(dst, v),
        Number2::IntNoSuffix(v) => write_i64(dst, v),

        Number2::UInt8(v) => write_u64(dst, v.into()),
        Number2::UInt16(v) => write_u64(dst, v.into()),
        Number2::UInt32(v) => write_u64(dst, v.into()),
        Number2::UInt64(v) => write_u64(dst, v),
        Number2::UIntNoSuffix(v) => write_u64(dst, v),

        Number2::Float32(Float32(v)) => write_f32(dst, v),
        Number2::Float64(Float64(v)) => write_f64(dst, v),
        Number2::FloatNoSuffix(Float64(v)) => write_f64(dst, v),

        Number2::Int128 { lo, hi } => write_i128(dst, (hi as i128) << 64 | lo as i128),
        Number2::UInt128 { lo, hi } => write_u128(dst, (hi as u128) << 64 | lo as u128),
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
