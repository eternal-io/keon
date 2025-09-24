use self::error::*;
use crate::{format::*, value::*};
use core::fmt::{self, Write};
use lexical_core::{FormattedSize, ToLexicalWithOptions, WriteIntegerOptions, WriteOptions};

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
}

impl<W: Write> Serria<W> {
    pub fn new(dst: W, style: Style) -> Self {
        Self::new_limited(dst, style, None)
    }

    pub fn new_limited(dst: W, style: Style, limit: Option<u32>) -> Self {
        Self { dst, style, limit }
    }
}

//------------------------------------------------------------------------------

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub pretty_depth: u8,
    pub pretty_width: u8,
    pub number_suffix: NumberSuffixStyle,
    pub struct_name: StructNameStyle,
    pub enum_name: EnumNameStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum NumberSuffixStyle {
    Retain,
    ExcludeFloat,
    ExcludeAll,
}

#[derive(Debug, Clone, Copy)]
pub enum StructNameStyle {
    /// `Struct { ... }`, `Tuple(_, _)`, `Newtype(_)`
    Retain,
    /// `Struct { ... }`, `Tuple(_, _)`, `_(_)`
    ExcludeNewtype,
    /// `Struct { ... }`, `_(_, _)`, `_(_)`
    ExcludeTuple,
    /// `_ { ... }`, `_(_, _)`, `_(_)`
    ExcludeAll,
}

#[derive(Debug, Clone, Copy)]
pub enum EnumNameStyle {
    /// `Enum::Variant { ... }`
    EnumToVariant,
    /// `_::Variant { ... }`
    ToVariant,
    /// `Variant { ... }`
    Variant,
}

//------------------------------------------------------------------------------

struct State {
    dep: u32,
    ttl: Option<u32>,
    buf: Vec<u8>,
}

impl State {
    fn get_buf(&mut self, size: usize) -> &mut [u8] {
        if self.buf.len() < size {
            self.buf.extend(core::iter::repeat_n(0, self.buf.len() - size));
        }

        &mut self.buf
    }
}

//------------------------------------------------------------------------------

impl<W: Write> Serria<W> {
    fn serialize<T: Seriable>(&mut self, val: T) -> SeriaResult {
        todo!()
    }
}

//------------------------------------------------------------------------------

trait HasName {
    const NAME: &str;
}

macro_rules! impl_has_name_for_primitive {
    ( $( $ty:ident ),* $(,)? ) => { $(
        impl HasName for $ty {
            const NAME: &str = stringify!($ty);
        }
    )* };
}

impl_has_name_for_primitive! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
             f32, f64,
}

impl<W: Write> Serria<W> {
    fn write_integer<'a, T>(&mut self, state: &'a mut State) -> SeriaResult<&'a mut [u8]>
    where
        T: HasName + FormattedSize + ToLexicalWithOptions<Options = WriteIntegerOptions>,
    {
        Ok(state.get_buf(WRITE_INTEGER_OPTS.buffer_size::<T, NUMBER_FORMAT>()))
    }
}

//------------------------------------------------------------------------------
