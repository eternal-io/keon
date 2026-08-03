use crate::{format::*, value::*, PrivateMethod};
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
    fn push_i64(&mut self, n: i64, suff: NumberSuffix) -> fmt::Result;
    fn push_i128(&mut self, n: i128) -> fmt::Result;
    fn push_u64(&mut self, n: u64, suff: NumberSuffix) -> fmt::Result;
    fn push_u128(&mut self, n: u128) -> fmt::Result;
    fn push_f32(&mut self, n: f32) -> fmt::Result;
    fn push_f64(&mut self, n: f64) -> fmt::Result;
    fn push_int(&mut self, n: i64) -> fmt::Result;
    fn push_uint(&mut self, n: u64) -> fmt::Result;
    fn push_float(&mut self, n: f64) -> fmt::Result;
    fn push_str(&mut self, s: &str) -> fmt::Result;
    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result;

    fn push_unit(&mut self) -> fmt::Result;
    fn push_unit_struct(&mut self, name: Option<&Ident>) -> fmt::Result;
    fn push_unit_variant(&mut self, name: Option<&Ident>, variant: &Ident) -> fmt::Result;

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
        Self::with_config(dst, SerializeConfig::minimal())
    }

    pub fn with_config(dst: W, cfg: SerializeConfig) -> Self {
        Self::with_config_and_limit(dst, cfg, 160)
    }

    pub fn with_config_and_limit(dst: W, cfg: SerializeConfig, recursion_limit: u16) -> Self {
        Self {
            ser: FastImpl::new(dst, cfg),
            ttl: recursion_limit,
        }
    }
}

impl<W: Write> Serializer<PrettyImpl<W>> {
    pub fn new_pretty(dst: W) -> Self {
        Self::with_config_pretty(dst, SerializeConfig::default())
    }

    pub fn with_config_pretty(dst: W, cfg: SerializeConfig) -> Self {
        Self::with_config_and_limit_pretty(dst, cfg, 160)
    }

    pub fn with_config_and_limit_pretty(dst: W, cfg: SerializeConfig, recursion_limit: u16) -> Self {
        Self {
            ser: PrettyImpl::new(dst, cfg),
            ttl: recursion_limit,
        }
    }
}

impl<Impl: SerializerImpl> Serializer<Impl> {
    #[inline]
    pub fn serialize<T>(&mut self, value: &T) -> fmt::Result
    where
        T: ?Sized + Serialize,
    {
        if self.ttl == 0 {
            return Err(fmt::Error);
        }
        self.ttl -= 1;
        value.serialize_with(self, PrivateMethod)?;
        self.ttl += 1;

        Ok(())
    }

    #[inline]
    pub fn serialize_many<T>(&mut self, values: impl IntoIterator<Item: AsRef<T>>) -> fmt::Result
    where
        T: ?Sized + Serialize,
    {
        values.into_iter().try_for_each(|value| self.serialize(value.as_ref()))
    }
}

//------------------------------------------------------------------------------

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct SerializeConfig {
    pub indentor: Indentor,
    pub numeric_suffix: NumericSuffix,
    pub nominal_path_style: NominalPathStyle,
    pub map_like_inline_entries: u8,
}

impl SerializeConfig {
    pub const fn minimal() -> Self {
        Self {
            indentor: Indentor::tab(),
            numeric_suffix: NumericSuffix::LongIntegerOnly,
            nominal_path_style: NominalPathStyle::Minimal,
            map_like_inline_entries: 3,
        }
    }
}

impl Default for SerializeConfig {
    fn default() -> Self {
        Self {
            indentor: Indentor::space(4),
            numeric_suffix: NumericSuffix::LongIntegerOnly,
            nominal_path_style: NominalPathStyle::Full,
            map_like_inline_entries: 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Indentor(u8);

impl Indentor {
    pub const fn tab() -> Self {
        Self(0)
    }

    pub const fn space(n_or_tab: u8) -> Self {
        Self(n_or_tab)
    }

    #[inline(always)]
    fn write_to(&self, dst: &mut impl Write) -> fmt::Result {
        match self.0 {
            0 => dst.write_str("\t"),
            n => (0..n).try_for_each(|_| dst.write_str(" ")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumericSuffix {
    Always = 0,
    IntegerOnly = 1,
    LongIntegerOnly = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NominalPathStyle {
    Full = 0,
    Named = 1,
    Minimal = 2,
}

//==================================================================================================

// enum Token<'a> {
//     #[cfg(feature = "alloc")]
//     Stringified(String),
//     Literal(Literal<'a>),
//     Ident(&'a str),
//     Unit,
//     UnitStruct {
//         kind: NominalKind,
//         path: NominalPathRef<'a>,
//     },

//     Maybe,
//     Array,
//     Tuple,
//     TupleStruct {
//         kind: NominalKind,
//         path: NominalPathRef<'a>,
//     },
//     Map,
//     MapStruct {
//         kind: NominalKind,
//         path: NominalPathRef<'a>,
//     },

//     MaybeEnd,
//     ArrayEnd,
//     TupleEnd,
//     MapLikeEnd,

//     FatArrow,
//     Colon,
//     Comma,
// }

enum Literal<'a> {
    Bool(bool),
    Char(char),
    Number(Number2),
    Str(&'a str),
    Bytes(&'a [u8]),
}

enum NominalKind {
    Unspecified,
    Variant,
    Struct,
}

enum RangeType {
    RangeTo,
    RangeToInclusive,
    RangeFrom,
    Range,
    RangeInclusive,
}

impl From<Scalar> for Literal<'_> {
    fn from(value: Scalar) -> Self {
        match value {
            Scalar::Char(ch) => Self::Char(ch),
            Scalar::Number(num) => Self::Number(num),
        }
    }
}

//------------------------------------------------------------------------------

pub struct FastImpl<W> {
    dst: W,
    cfg: SerializeConfig,
    lvl: u16,
    range_hook: Option<RangeType>,
    range_start: Option<Option<Scalar>>,
    range_end: Option<Option<Scalar>>,
}

impl<W> FastImpl<W> {
    fn new(dst: W, cfg: SerializeConfig) -> Self {
        Self {
            dst,
            cfg,
            lvl: 0,
            range_hook: None,
            range_start: None,
            range_end: None,
        }
    }
}

impl<W: Write> FastImpl<W> {
    fn release_range_hook(&mut self) -> fmt::Result {
        if let Some(typ) = self.range_hook.take() {
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
                write_literal(&mut self.dst, scalar.into(), self.cfg.numeric_suffix)?;
                self.dst.write_str(",")?;
            }
        }
        if let Some(end) = self.range_end.take() {
            self.dst.write_str("end")?;
            if let Some(scalar) = end {
                self.dst.write_str(":")?;
                write_literal(&mut self.dst, scalar.into(), self.cfg.numeric_suffix)?;
                self.dst.write_str(",")?;
            }
        }
        Ok(())
    }

    fn semicolon(&mut self) -> fmt::Result {
        if self.lvl == 0 {
            self.dst.write_str(";")?;
        }
        Ok(())
    }
}

impl<W> Drop for FastImpl<W> {
    fn drop(&mut self) {
        debug_assert!(self.lvl == 0);
    }
}

impl<W: Write> SerializerImpl for FastImpl<W> {}

impl<W: Write> SerializerImplDetail for FastImpl<W> {
    /*
    #[doc(hidden)]
    fn push(&mut self, token: Token<'_>) -> fmt::Result {
        if matches!(
            token,
            Token::Maybe
                | Token::Array
                | Token::Tuple
                | Token::TupleStruct { .. }
                | Token::Map
                | Token::MapStruct { .. }
        ) {
            self.lvl += 1;
        }

        if matches!(
            token,
            Token::MaybeEnd | Token::ArrayEnd | Token::TupleEnd | Token::MapLikeEnd
        ) {
            self.lvl -= 1;
        }

        'value: {
            if let Some(ref typ) = self.range_hook {
                'range: {
                    match token {
                        Token::Ident(ident) => {
                            if ident == "start"
                                && matches!(typ, RangeType::RangeFrom)
                                && matches!(typ, RangeType::Range | RangeType::RangeInclusive)
                            {
                                self.range_start = Some(None);
                            } else if ident == "end"
                                && matches!(typ, RangeType::RangeTo | RangeType::RangeToInclusive)
                                && matches!(typ, RangeType::Range | RangeType::RangeInclusive)
                            {
                                self.range_end = Some(None);
                            } else {
                                break 'range;
                            }
                        }
                        Token::Literal(ref literal) => {
                            let scalar = match literal {
                                Literal::Char(ch) => Scalar::Char(*ch),
                                Literal::Number(num) => Scalar::Number(*num),
                                _ => break 'range,
                            };
                            match (self.range_start, self.range_end) {
                                (Some(None), None | Some(Some(_))) => self.range_start = Some(Some(scalar)),
                                (None | Some(Some(_)), Some(None)) => self.range_end = Some(Some(scalar)),
                                _ => panic!("contract violation"),
                            }
                        }
                        Token::MapLikeEnd => {
                            match typ {
                                RangeType::RangeTo => {
                                    if let (None, Some(Some(end))) = (self.range_start, self.range_end) {
                                        self.dst.write_str("..")?;
                                        write_literal(&mut self.dst, end.into(), self.cfg.numeric_suffix)?;
                                    } else {
                                        break 'range;
                                    }
                                }
                                RangeType::RangeToInclusive => {
                                    if let (None, Some(Some(end))) = (self.range_start, self.range_end) {
                                        self.dst.write_str("..=")?;
                                        write_literal(&mut self.dst, end.into(), self.cfg.numeric_suffix)?;
                                    } else {
                                        break 'range;
                                    }
                                }
                                RangeType::RangeFrom => {
                                    if let (Some(Some(start)), None) = (self.range_start, self.range_end) {
                                        write_literal(&mut self.dst, start.into(), self.cfg.numeric_suffix)?;
                                        self.dst.write_str("..")?;
                                    } else {
                                        break 'range;
                                    }
                                }
                                RangeType::Range => {
                                    if let (Some(Some(start)), Some(Some(end))) = (self.range_start, self.range_end) {
                                        write_literal(&mut self.dst, start.into(), self.cfg.numeric_suffix)?;
                                        self.dst.write_str("..")?;
                                        write_literal(&mut self.dst, end.into(), self.cfg.numeric_suffix)?;
                                    } else {
                                        break 'range;
                                    }
                                }
                                RangeType::RangeInclusive => {
                                    if let (Some(Some(start)), Some(Some(end))) = (self.range_start, self.range_end) {
                                        write_literal(&mut self.dst, start.into(), self.cfg.numeric_suffix)?;
                                        self.dst.write_str("..=")?;
                                        write_literal(&mut self.dst, end.into(), self.cfg.numeric_suffix)?;
                                    } else {
                                        break 'range;
                                    }
                                }
                            };
                            self.range_hook = None;
                            self.range_start = None;
                            self.range_end = None;
                        }
                        _ => break 'range,
                    }
                    break 'value;
                }
                self.release_range_hook()?;
            }

            match token {
                #[cfg(feature = "alloc")]
                Token::Stringified(_) => panic!("FastImpl does not rely on alloc"),
                Token::Literal(literal) => write_literal(&mut self.dst, literal, self.cfg.numeric_suffix)?,
                Token::Ident(ident) => self.dst.write_str(ident)?,
                Token::Unit => self.dst.write_str("()")?,
                Token::UnitStruct { kind, path } => {
                    write_nominal_path(&mut self.dst, kind, path, self.cfg.nominal_path_style)?
                }

                Token::Maybe => self.dst.write_str("?")?,
                Token::Array => self.dst.write_str("[")?,
                Token::Tuple | Token::TupleStruct { .. } => {
                    if let Token::TupleStruct { kind, path } = token {
                        write_nominal_path(&mut self.dst, kind, path, self.cfg.nominal_path_style)?;
                    }
                    self.dst.write_str("(")?;
                }
                Token::Map | Token::MapStruct { .. } => 'map_like: {
                    if let Token::MapStruct { kind, path } = token {
                        'range_type: {
                            let NominalPathRef::Single { name } = path else {
                                break 'range_type;
                            };
                            let typ = match name.as_str() {
                                "RangeTo" => RangeType::RangeTo,
                                "RangeToInclusive" => RangeType::RangeToInclusive,
                                "RangeFrom" => RangeType::RangeFrom,
                                "Range" => RangeType::Range,
                                "RangeInclusive" => RangeType::RangeInclusive,
                                _ => break 'range_type,
                            };

                            self.range_hook = Some(typ);

                            break 'map_like;
                        }
                        write_nominal_path(&mut self.dst, kind, path, self.cfg.nominal_path_style)?;
                    }
                    self.dst.write_str("{")?;
                }

                Token::MaybeEnd => (),
                Token::ArrayEnd => self.dst.write_str("]")?,
                Token::TupleEnd => self.dst.write_str(")")?,
                Token::MapLikeEnd => self.dst.write_str("}")?,

                Token::FatArrow => self.dst.write_str("=>")?,
                Token::Colon => self.dst.write_str(":")?,
                Token::Comma => self.dst.write_str(",")?,
            }
        }

        if self.lvl == 0 {
            self.dst.write_str(";")?;
        }

        Ok(())
    }
    */

    fn push_stringified(&mut self, _stringified: String) -> fmt::Result {
        panic!("FastImpl does not rely on alloc")
    }

    fn push_range_full(&mut self) -> fmt::Result {
        self.dst.write_str("..")?;
        self.semicolon()
    }
    fn push_range_to(&mut self, end: &Scalar) -> fmt::Result {
        self.dst.write_str("..")?;
        write_scalar(&mut self.dst, end, self.cfg.numeric_suffix)?;
        self.semicolon()
    }
    fn push_range_to_inclusive(&mut self, end: &Scalar) -> fmt::Result {
        self.dst.write_str("..=")?;
        write_scalar(&mut self.dst, end, self.cfg.numeric_suffix)?;
        self.semicolon()
    }
    fn push_range_from(&mut self, start: &Scalar) -> fmt::Result {
        write_scalar(&mut self.dst, start, self.cfg.numeric_suffix)?;
        self.dst.write_str("..")?;
        self.semicolon()
    }
    fn push_range(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        write_scalar(&mut self.dst, start, self.cfg.numeric_suffix)?;
        self.dst.write_str("..")?;
        write_scalar(&mut self.dst, end, self.cfg.numeric_suffix)?;
        self.semicolon()
    }
    fn push_range_inclusive(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        write_scalar(&mut self.dst, start, self.cfg.numeric_suffix)?;
        self.dst.write_str("..=")?;
        write_scalar(&mut self.dst, end, self.cfg.numeric_suffix)?;
        self.semicolon()
    }

    fn push_bool(&mut self, b: bool) -> fmt::Result {
        todo!()
    }

    fn push_char(&mut self, ch: char) -> fmt::Result {
        todo!()
    }

    fn push_i64(&mut self, n: i64, suff: NumberSuffix) -> fmt::Result {
        todo!()
    }

    fn push_i128(&mut self, n: i128) -> fmt::Result {
        todo!()
    }

    fn push_u64(&mut self, n: u64, suff: NumberSuffix) -> fmt::Result {
        todo!()
    }

    fn push_u128(&mut self, n: u128) -> fmt::Result {
        todo!()
    }

    fn push_f32(&mut self, n: f32) -> fmt::Result {
        todo!()
    }

    fn push_f64(&mut self, n: f64) -> fmt::Result {
        todo!()
    }

    fn push_int(&mut self, n: i64) -> fmt::Result {
        todo!()
    }

    fn push_uint(&mut self, n: u64) -> fmt::Result {
        todo!()
    }

    fn push_float(&mut self, n: f64) -> fmt::Result {
        todo!()
    }

    fn push_str(&mut self, s: &str) -> fmt::Result {
        todo!()
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
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
}

//------------------------------------------------------------------------------

#[cfg(feature = "alloc")]
pub struct PrettyImpl<W> {
    dst: W,
    cfg: SerializeConfig,
    compounds_stack: Vec<Compound>,
    inline_entries: VecDeque<String>,
}

#[cfg(feature = "alloc")]
impl<W> PrettyImpl<W> {
    fn new(dst: W, cfg: SerializeConfig) -> Self {
        Self {
            dst,
            cfg,
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

    fn push_range_full(&mut self) -> fmt::Result {
        let stringified = String::from("..");
        self.push_stringified(stringified)
    }
    fn push_range_to(&mut self, end: &Scalar) -> fmt::Result {
        let mut stringified = String::from("..");
        write_scalar(&mut stringified, end, self.cfg.numeric_suffix)?;
        self.push_stringified(stringified)
    }
    fn push_range_to_inclusive(&mut self, end: &Scalar) -> fmt::Result {
        let mut stringified = String::from("..=");
        write_scalar(&mut stringified, end, self.cfg.numeric_suffix)?;
        self.push_stringified(stringified)
    }
    fn push_range_from(&mut self, start: &Scalar) -> fmt::Result {
        let mut stringified = String::new();
        write_scalar(&mut stringified, start, self.cfg.numeric_suffix)?;
        stringified.push_str("..");
        self.push_stringified(stringified)
    }
    fn push_range(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        let mut stringified = String::new();
        write_scalar(&mut stringified, start, self.cfg.numeric_suffix)?;
        stringified.push_str("..");
        write_scalar(&mut stringified, end, self.cfg.numeric_suffix)?;
        self.push_stringified(stringified)
    }
    fn push_range_inclusive(&mut self, start: &Scalar, end: &Scalar) -> fmt::Result {
        let mut stringified = String::new();
        write_scalar(&mut stringified, start, self.cfg.numeric_suffix)?;
        stringified.push_str("..=");
        write_scalar(&mut stringified, end, self.cfg.numeric_suffix)?;
        self.push_stringified(stringified)
    }

    fn push_bool(&mut self, b: bool) -> fmt::Result {
        todo!()
    }

    fn push_char(&mut self, ch: char) -> fmt::Result {
        todo!()
    }

    fn push_i64(&mut self, n: i64, suff: NumberSuffix) -> fmt::Result {
        todo!()
    }

    fn push_i128(&mut self, n: i128) -> fmt::Result {
        todo!()
    }

    fn push_u64(&mut self, n: u64, suff: NumberSuffix) -> fmt::Result {
        todo!()
    }

    fn push_u128(&mut self, n: u128) -> fmt::Result {
        todo!()
    }

    fn push_f32(&mut self, n: f32) -> fmt::Result {
        todo!()
    }

    fn push_f64(&mut self, n: f64) -> fmt::Result {
        todo!()
    }

    fn push_int(&mut self, n: i64) -> fmt::Result {
        todo!()
    }

    fn push_uint(&mut self, n: u64) -> fmt::Result {
        todo!()
    }

    fn push_float(&mut self, n: f64) -> fmt::Result {
        todo!()
    }

    fn push_str(&mut self, s: &str) -> fmt::Result {
        todo!()
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> fmt::Result {
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
}

//==================================================================================================

enum Numeric {
    Int(i64),
    UInt(u64),
    Float32(f32),
    Float64(f64),
    LongInt { lo: u64, hi: i64 },
    LongUInt { lo: u64, hi: u64 },
}

impl From<Number2> for Numeric {
    fn from(value: Number2) -> Self {
        match value {
            Number2::Int8(x) => Numeric::Int(x as _),
            Number2::Int16(x) => Numeric::Int(x as _),
            Number2::Int32(x) => Numeric::Int(x as _),
            Number2::Int64(x) => Numeric::Int(x),
            Number2::Int128 { lo, hi } => Numeric::LongInt { lo, hi },
            Number2::UInt8(x) => Numeric::UInt(x as _),
            Number2::UInt16(x) => Numeric::UInt(x as _),
            Number2::UInt32(x) => Numeric::UInt(x as _),
            Number2::UInt64(x) => Numeric::UInt(x),
            Number2::UInt128 { lo, hi } => Numeric::LongUInt { lo, hi },
            Number2::Float32(Float32(x)) => Numeric::Float32(x),
            Number2::Float64(Float64(x)) => Numeric::Float64(x),

            Number2::IntNoSuffix(x) => Numeric::Int(x),
            Number2::UIntNoSuffix(x) => Numeric::UInt(x),
            Number2::FloatNoSuffix(Float64(x)) => Numeric::Float64(x),
        }
    }
}

fn write_number(dst: &mut impl Write, number: Number2, suffix_control: NumericSuffix) -> fmt::Result {
    let mut buf = [0x00u8; lexical_core::BUFFER_SIZE];

    macro_rules! write_number_case {
        ($x:expr, $buf:ident, $write_options:path) => {
            dst.write_str(unsafe {
                core::str::from_utf8_unchecked(lexical_core::write_with_options::<_, NUMBER_FORMAT>(
                    $x,
                    &mut $buf,
                    &$write_options,
                ))
            })?
        };
    }
    match Numeric::from(number) {
        Numeric::Int(x) => write_number_case!(x, buf, WRITE_INTEGER_OPTS),
        Numeric::UInt(x) => write_number_case!(x, buf, WRITE_INTEGER_OPTS),
        Numeric::Float32(x) => write_number_case!(x, buf, WRITE_FLOAT_OPTS),
        Numeric::Float64(x) => write_number_case!(x, buf, WRITE_FLOAT_OPTS),
        Numeric::LongInt { lo, hi } => write_number_case!((hi as i128) << 64 | lo as i128, buf, WRITE_INTEGER_OPTS),
        Numeric::LongUInt { lo, hi } => write_number_case!((hi as u128) << 64 | lo as u128, buf, WRITE_INTEGER_OPTS),
    }

    if matches!(
        number,
        Number2::IntNoSuffix(_) | Number2::UIntNoSuffix(_) | Number2::FloatNoSuffix(_)
    ) {
        return Ok(());
    }
    match number {
        Number2::Int128 { .. } => dst.write_str("i128"),
        Number2::UInt128 { .. } => dst.write_str("u128"),

        _ => match suffix_control <= NumericSuffix::IntegerOnly {
            true => match number {
                Number2::Int8(_) => dst.write_str("i8"),
                Number2::Int16(_) => dst.write_str("i16"),
                Number2::Int32(_) => dst.write_str("i32"),
                Number2::Int64(_) => dst.write_str("i64"),
                Number2::UInt8(_) => dst.write_str("u8"),
                Number2::UInt16(_) => dst.write_str("u16"),
                Number2::UInt32(_) => dst.write_str("u32"),
                Number2::UInt64(_) => dst.write_str("u64"),

                _ => match suffix_control <= NumericSuffix::Always {
                    true => match number {
                        Number2::Float32(_) => dst.write_str("f32"),
                        Number2::Float64(_) => dst.write_str("f64"),

                        _ => unreachable!(),
                    },
                    false => Ok(()),
                },
            },
            false => Ok(()),
        },
    }
}

//------------------------------------------------------------------------------

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextualKind {
    Char,
    String,
}

#[inline(always)]
fn write_escaped_byte(dst: &mut impl Write, byte: u8, ctx: TextualKind) -> fmt::Result {
    match byte {
        b'\0' => dst.write_str(r#"\0"#),
        b'\n' => dst.write_str(r#"\n"#),
        b'\t' => dst.write_str(r#"\t"#),
        b'\r' => dst.write_str(r#"\r"#),
        b'\'' if ctx == TextualKind::Char => dst.write_str(r#"\'"#),
        b'\"' if ctx == TextualKind::String => dst.write_str(r#"\""#),
        0x20..=0x7e => dst.write_char(byte.into()),
        _ => {
            dst.write_str(r#"\x"#)?;
            write_u8_fmt_02_hex(dst, byte)
        }
    }
}

#[inline(always)]
fn write_escaped_char(dst: &mut impl Write, ch: char, ctx: TextualKind) -> fmt::Result {
    match ch {
        '\0' => dst.write_str(r#"\0"#),
        '\n' => dst.write_str(r#"\n"#),
        '\t' => dst.write_str(r#"\t"#),
        '\r' => dst.write_str(r#"\r"#),
        '\'' if ctx == TextualKind::Char => dst.write_str(r#"\'"#),
        '\"' if ctx == TextualKind::String => dst.write_str(r#"\""#),
        '\x01'..='\x19' | '\x7f' => {
            dst.write_str(r#"\x"#)?;
            write_u8_fmt_02_hex(dst, ch as u8)
        }
        _ => dst.write_char(ch),
    }
}

#[inline(always)]
fn write_u8_fmt_02_hex(dst: &mut impl Write, byte: u8) -> fmt::Result {
    const NUMBER_FORMAT_HEX_NO_PREFIX: u128 = lexical_core::NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(16)
        .build_strict();

    let mut buf = [b'0'; 2];

    lexical_core::write_with_options::<u8, NUMBER_FORMAT_HEX_NO_PREFIX>(
        byte,
        &mut buf[(byte < 0x10) as usize..],
        &WRITE_INTEGER_OPTS,
    );

    dst.write_str(unsafe { ::core::str::from_utf8_unchecked(&buf) })
}

fn write_quoted_char(dst: &mut impl Write, ch: char) -> fmt::Result {
    dst.write_str(r#"'"#)?;
    write_escaped_char(dst, ch, TextualKind::Char)?;
    dst.write_str(r#"'"#)
}

fn write_quoted_string(dst: &mut impl Write, s: &str) -> fmt::Result {
    dst.write_str(r#"""#)?;
    s.chars()
        .try_for_each(|ch| write_escaped_char(dst, ch, TextualKind::String))?;
    dst.write_str(r#"""#)
}

fn write_quoted_bytes(dst: &mut impl Write, bytes: &[u8]) -> fmt::Result {
    dst.write_str(r#"b""#)?;
    bytes
        .into_iter()
        .try_for_each(|&byte| write_escaped_byte(dst, byte, TextualKind::String))?;
    dst.write_str(r#"""#)
}

//------------------------------------------------------------------------------

fn write_nominal_path(
    dst: &mut impl Write,
    kind: NominalKind,
    path: NominalPathRef<'_>,
    style: NominalPathStyle,
) -> fmt::Result {
    use {NominalKind as Kind, NominalPathRef as Path, NominalPathStyle as Style};

    match path {
        Path::Dual { name, parent } => {
            if matches!(kind, Kind::Unspecified) || style <= Style::Full {
                dst.write_str(&parent)?;
                dst.write_str("::")?;
                dst.write_str(&name)
            } else if matches!(kind, Kind::Variant) || style <= Style::Named {
                dst.write_str(&name)
            } else {
                dst.write_str("_")
            }
        }
        Path::Single { name } => {
            if matches!(kind, Kind::Unspecified | Kind::Variant) || style <= Style::Named {
                dst.write_str(&name)
            } else {
                dst.write_str("_")
            }
        }
        Path::Underscore => {
            if matches!(kind, Kind::Unspecified | Kind::Struct) {
                dst.write_str("_")
            } else {
                panic!("missing variant name")
            }
        }
    }
}

fn write_literal(dst: &mut impl Write, literal: Literal<'_>, suffix_control: NumericSuffix) -> fmt::Result {
    match literal {
        Literal::Bool(b) => match b {
            true => dst.write_str("true"),
            false => dst.write_str("false"),
        },
        Literal::Char(ch) => write_quoted_char(dst, ch),
        Literal::Number(num) => write_number(dst, num, suffix_control),
        Literal::Str(s) => write_quoted_string(dst, s),
        Literal::Bytes(bytes) => write_quoted_bytes(dst, bytes),
    }
}

fn write_scalar(dst: &mut impl Write, scalar: &Scalar, suffix_control: NumericSuffix) -> fmt::Result {
    match scalar {
        Scalar::Char(ch) => write_quoted_char(dst, *ch),
        Scalar::Number(num) => write_number(dst, *num, suffix_control),
    }
}
