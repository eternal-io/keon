use self::error::*;
use crate::{format::*, value::*};
use core::fmt::{self, Write};
use lexical_core::{ToLexicalWithOptions, WriteFloatOptions, WriteIntegerOptions, WriteOptions};

pub mod error;
pub mod ser_concr;
pub mod ser_value;

#[doc(alias = "compact_seria")]
pub fn seria<T: Seriable>(value: T) -> String {
    todo!()
}

#[doc(alias = "compact_seria_many")]
pub fn seria_many<T, I>(values: I)
where
    T: Seriable,
    I: Iterator<Item = T>,
{
    todo!()
}

pub fn pretty_seria<T: Seriable>(value: T) -> String {
    todo!()
}

pub fn custom_seria<T: Seriable>(value: T, style: Style) -> String {
    todo!()
}

//------------------------------------------------------------------------------

#[doc(alias = "Serialize")]
pub trait Seriable {
    fn seria_via<W: Write>(&self, ser: &mut Serria<W>) -> SeriaResult;
}

//------------------------------------------------------------------------------

#[doc(alias = "Serializer")]
pub struct Serria<W: Write> {
    dst: W,
    style: Style,
    limit: Option<u32>,
    buf: Vec<u8>,
}

impl<W: Write> Serria<W> {
    pub fn new(dst: W, style: Style) -> Self {
        Self::new_limited(dst, style, None)
    }

    pub fn new_limited(dst: W, style: Style, limit: Option<u32>) -> Self {
        Self {
            dst,
            style,
            limit,
            buf: Vec::new(),
        }
    }
}

//------------------------------------------------------------------------------

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub pretty_depth: u8,
    pub bytes_flavor: BytesFlavor,
    pub number_suffix: NumberSuffixStyle,
    pub struct_name: StructNameStyle,
    pub enum_name: EnumNameStyle,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum BytesFlavor {
    #[default]
    Normal,
    Base64,
    Base32,
    Base16,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum NumberSuffixStyle {
    Retain,
    #[default]
    ExcludeFloat,
    ExcludeAll,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum StructNameStyle {
    /// `Struct { ... }`, `Tuple(_, _)`, `Newtype(_)`
    #[default]
    Retain,
    /// `Struct { ... }`, `Tuple(_, _)`, `_(_)`
    ExcludeNewtype,
    /// `Struct { ... }`, `_(_, _)`, `_(_)`
    ExcludeTuple,
    /// `_ { ... }`, `_(_, _)`, `_(_)`
    ExcludeAll,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum EnumNameStyle {
    /// `Enum::Variant { ... }`
    #[default]
    EnumToVariant,
    /// `_::Variant { ... }`
    ToVariant,
    /// `Variant { ... }`
    Variant,
}

impl Style {
    pub fn new() -> Self {
        Self {
            pretty_depth: 6,
            ..Default::default()
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

//------------------------------------------------------------------------------

impl<W: Write> Serria<W> {
    fn get_buf_dst(&mut self, size: usize) -> (&mut [u8], &mut W) {
        if self.buf.len() < size {
            self.buf.extend(core::iter::repeat_n(0, self.buf.len() - size));
        }

        (&mut self.buf, &mut self.dst)
    }
}

//------------------------------------------------------------------------------

trait NumberInfo {
    const SUFFIX: &str;
    const BITS: usize;
}

macro_rules! impl_has_name_for_primitive {
    ( $( $ty:ident ),* $(,)? ) => { $(
        impl NumberInfo for $ty {
            const SUFFIX: &str = stringify!($ty);
            const BITS: usize = size_of::<$ty>() * 8;
        }
    )* };
}

impl_has_name_for_primitive! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
             f32, f64,
}

impl<W: Write> Serria<W> {
    fn write_integer<T>(&mut self, val: T) -> SeriaResult
    where
        T: NumberInfo + ToLexicalWithOptions<Options = WriteIntegerOptions>,
    {
        let write_suffix = matches!(
            self.style.number_suffix,
            NumberSuffixStyle::Retain | NumberSuffixStyle::ExcludeFloat
        ) || T::BITS == 128;

        let (buf, dst) = self.get_buf_dst(WRITE_INTEGER_OPTS.buffer_size::<T, NUMBER_FORMAT>());
        let num = lexical_core::write_with_options::<T, NUMBER_FORMAT>(val, buf, &WRITE_INTEGER_OPTS);
        let num = unsafe { core::str::from_utf8_unchecked(num) };

        dst.write_str(num)?;
        if write_suffix {
            dst.write_str(T::SUFFIX)?;
        }

        Ok(())
    }

    fn write_float<T>(&mut self, val: T) -> SeriaResult
    where
        T: NumberInfo + ToLexicalWithOptions<Options = WriteFloatOptions>,
    {
        let write_suffix = matches!(self.style.number_suffix, NumberSuffixStyle::Retain);

        let (buf, dst) = self.get_buf_dst(WRITE_FLOAT_OPTS.buffer_size::<T, NUMBER_FORMAT>());
        let num = lexical_core::write_with_options::<T, NUMBER_FORMAT>(val, buf, &WRITE_FLOAT_OPTS);
        let num = unsafe { core::str::from_utf8_unchecked(num) };

        dst.write_str(num)?;
        if write_suffix {
            dst.write_str(T::SUFFIX)?;
        }

        Ok(())
    }
}

//------------------------------------------------------------------------------

impl<W: Write> Serria<W> {
    fn write_bool(&mut self, val: bool) -> SeriaResult {
        match val {
            true => self.dst.write_str("true")?,
            false => self.dst.write_str("false")?,
        }

        Ok(())
    }

    fn write_char(&mut self, val: char) -> SeriaResult {
        todo!()
    }

    fn write_str(&mut self, val: &str) -> SeriaResult {
        todo!()
    }

    fn write_bytes(&mut self, val: &[u8]) -> SeriaResult {
        todo!()
    }
}

//------------------------------------------------------------------------------

enum Container {
    Tuple,
    Seq,
    Map,
    Struct,
}

enum NominalContainer {
    Tuple,
    Struct,
}

impl Container {
    fn enter<'a, W: Write>(&self, ser: &'a mut Serria<W>) -> SeriaResult<SerriaEntry<'a, W>> {
        match self {
            Container::Tuple => todo!(),
            Container::Seq => todo!(),
            Container::Map => todo!(),
            Container::Struct => todo!(),
        }
    }
}

#[doc(hidden)]
pub struct SerriaEntry<'a, W: Write> {
    ser: &'a mut Serria<W>,
    typ: Container,
}

impl<'a, W: Write> SerriaEntry<'a, W> {
    fn serialize<T: Seriable>(&mut self, val: T) -> SeriaResult {
        todo!()
    }

    fn leave(&mut self) -> SeriaResult {
        todo!()
    }
}
