use self::{options::*, private::*};
use crate::{
    format,
    value::{self, Float32, Float64},
};
use alloc::collections::VecDeque;
use core::fmt::{self, Write};
use either::Either;

//==================================================================================================

pub trait Serialize {
    fn seria_with(&self, ser: impl Serializer) -> fmt::Result;
}

pub trait Serializer {
    fn push(&mut self, token: Token<'_>) -> fmt::Result;
}

mod private {
    use crate::value::Number;

    pub enum Token<'a> {
        #[cfg(feature = "alloc")]
        Stringified(String),
        Literal(Literal<'a>),
        Maybe,
        Sequence,
        TupleLike(Option<NominalPath<'a>>),
        MapLike(Option<NominalPath<'a>>),
        End,
    }

    pub enum Literal<'a> {
        Bool(bool),
        Char(char),
        Num(Number),
        Str(&'a str),
        Bytes(&'a [u8]),
    }

    #[derive(Clone, Copy)]
    pub enum NominalPath<'a> {
        Unspecified,
        Single { name: &'a str },
        Dual { name: &'a str, parent: &'a str },
    }
}

//==================================================================================================

#[doc(alias = "Serializer")]
pub struct FastSerria<W: Write> {
    dst: W,
    cfg: FastSerriaConfig,
}

pub struct FastSerriaConfig {}

//==================================================================================================

#[doc(alias = "Serializer")]
pub struct Serria<W: Write> {
    dst: W,
    cfg: SerriaConfig,
    compounds_stack: Vec<Compound>,
    inline_entries: VecDeque<String>,
}

pub struct SerriaConfig {
    indent_width: u8,
    map_like_inline_entries: u8,
}

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

impl Compound {
    fn kind(&self) -> &CompoundKind {
        match self {
            Compound::Compact { kind, .. } | Compound::Expanded { kind } => kind,
        }
    }
}

enum CompoundKind {
    Maybe,
    Sequence,
    TupleLike(Option<String>),
    MapLikeLhs(Option<String>),
    MapLikeRhs(Option<String>),
}

impl CompoundKind {
    fn single_line_child(&self) -> bool {
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
            CompoundKind::Sequence => CompoundKind::Sequence,
            CompoundKind::TupleLike(name) => CompoundKind::TupleLike(name.take()),
            CompoundKind::MapLikeLhs(name) => CompoundKind::MapLikeLhs(name.take()),
            CompoundKind::MapLikeRhs(name) => CompoundKind::MapLikeRhs(name.take()),
        }
    }

    fn write_indicator(&self, dst: &mut impl Write) -> fmt::Result {
        match self {
            CompoundKind::Maybe => dst.write_str("?"),
            CompoundKind::Sequence => dst.write_str("["),
            CompoundKind::TupleLike(name) => {
                if let Some(name) = name {
                    dst.write_str(name)?;
                }
                dst.write_str("(")
            }
            CompoundKind::MapLikeLhs(name) | CompoundKind::MapLikeRhs(name) => {
                if let Some(name) = name {
                    dst.write_str(name)?;
                    dst.write_str(" ")?;
                }
                dst.write_str("{")
            }
        }
    }

    fn write_terminator(&self, dst: &mut impl Write) -> fmt::Result {
        match self {
            CompoundKind::Maybe => Ok(()),
            CompoundKind::Sequence => dst.write_str("]"),
            CompoundKind::TupleLike(_) => dst.write_str(")"),
            CompoundKind::MapLikeLhs(_) | CompoundKind::MapLikeRhs(_) => dst.write_str("}"),
        }
    }
}

impl<W: Write> Serializer for &mut Serria<W> {
    fn push(&mut self, token: Token) -> fmt::Result {
        let dst = &mut self.dst;
        let direct_write_indent = |dst: &mut W, depth: usize| -> fmt::Result {
            (0..self.cfg.indent_width as usize * depth).try_for_each(|_| dst.write_str(" "))
        };

        let direct_write_indicator =
            |dst: &mut W, depth: usize, single_line_child: bool, kind: &CompoundKind| -> fmt::Result {
                match single_line_child {
                    true => direct_write_indent(dst, depth)?,
                    false => dst.write_str(" ")?,
                }
                kind.write_indicator(dst)?;
                match kind.is_collection() {
                    true => dst.write_str("\n"),
                    false => Ok(()),
                }
            };

        let direct_write_entry = |dst: &mut W, depth: usize, kind: &mut CompoundKind, entry: &str| -> fmt::Result {
            match kind.single_line_child() {
                true => direct_write_indent(dst, depth)?,
                false => dst.write_str(" ")?,
            }
            dst.write_str(entry)?;
            match kind {
                CompoundKind::MapLikeLhs(name @ None) => {
                    *kind = CompoundKind::MapLikeRhs(name.take());
                    dst.write_str(" =>")
                }
                CompoundKind::MapLikeLhs(name @ Some(_)) => {
                    *kind = CompoundKind::MapLikeRhs(name.take());
                    dst.write_str(":")
                }
                CompoundKind::MapLikeRhs(name) => {
                    *kind = CompoundKind::MapLikeLhs(name.take());
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
                let mut single_line_child;
                for i in 0..compounds_count {
                    if let Compound::Expanded { .. } = compounds_stack[i] {
                        continue;
                    }

                    single_line_child = i
                        .checked_sub(1)
                        .map(|i| compounds_stack[i].kind().single_line_child())
                        .unwrap_or(false);

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

                    direct_write_indicator(dst, i, single_line_child, kind)?;
                    inline_entries
                        .drain(..range_end - range_start)
                        .try_for_each(|entry| direct_write_entry(dst, i + 1, kind, &entry))?;

                    compounds_stack[i] = Compound::Expanded { kind: kind.take() };
                }
                Ok(())
            };

        match token {
            Token::Literal(_) => todo!(),
            Token::Stringified(entry) => {
                match self.compounds_stack.last() {
                    Some(comp) => match comp {
                        Compound::Compact {
                            kind,
                            force_compact,
                            inline_entries_index,
                        } => {
                            self.inline_entries.push_back(entry.to_owned());
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

            Token::End => match self.compounds_stack.pop().unwrap() {
                Compound::Compact {
                    kind,
                    inline_entries_index,
                    ..
                } => {
                    let entries_count = self.inline_entries.len() - inline_entries_index;
                    let mut entries = self.inline_entries.drain(inline_entries_index..);
                    let mut stringified = String::new();

                    kind.write_indicator(&mut stringified)?;
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
                    kind.write_terminator(&mut stringified)?;
                    drop(entries);

                    self.push(Token::Stringified(stringified))?;
                }
                Compound::Expanded { kind } => {
                    direct_write_indent(dst, self.compounds_stack.len())?;

                    kind.write_terminator(dst)?;

                    match self.compounds_stack.last() {
                        Some(comp) => match comp.kind().is_collection() {
                            true => dst.write_str(",\n")?,
                            false => (),
                        },
                        None => dst.write_str(";\n")?,
                    }
                }
            },

            token => {
                let kind = match token {
                    Token::Maybe => CompoundKind::Maybe,
                    Token::Sequence => CompoundKind::Sequence,
                    // Token::TupleLike(name) => CompoundKind::TupleLike(name.map(ToOwned::to_owned)),
                    // Token::MapLike(name) => CompoundKind::MapLikeLhs(name.map(ToOwned::to_owned)),
                    // _ => unreachable!(),
                    _ => todo!(),
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
}

//==================================================================================================

pub mod options {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum NumericSuffix {
        Always = 0,
        IntegerOnly = 1,
        #[default]
        LongIntegerOnly = 2,
    }

    #[derive(Default, Debug, Clone, Copy)]
    pub enum NominalPathPolicy {
        #[default]
        Full,
        Named,
        Minimal,
    }
}

//------------------------------------------------------------------------------

enum Numeric {
    Int(i64),
    UInt(u64),
    LongInt(i128),
    LongUInt(u128),
    Float32(f32),
    Float64(f64),
}

impl From<&value::Number> for Numeric {
    fn from(value: &value::Number) -> Self {
        match value {
            value::Number::Int8(x) => Numeric::Int(*x as _),
            value::Number::Int16(x) => Numeric::Int(*x as _),
            value::Number::Int32(x) => Numeric::Int(*x as _),
            value::Number::Int64(x) => Numeric::Int(*x as _),
            value::Number::Int128(x) => Numeric::LongInt(**x),
            value::Number::UInt8(x) => Numeric::UInt(*x as _),
            value::Number::UInt16(x) => Numeric::UInt(*x as _),
            value::Number::UInt32(x) => Numeric::UInt(*x as _),
            value::Number::UInt64(x) => Numeric::UInt(*x as _),
            value::Number::UInt128(x) => Numeric::LongUInt(**x),
            value::Number::Float32(Float32(x)) => Numeric::Float32(*x),
            value::Number::Float64(Float64(x)) => Numeric::Float64(*x),
        }
    }
}

impl From<&value::NumberNoSuffix> for Numeric {
    fn from(value: &value::NumberNoSuffix) -> Self {
        match value {
            value::NumberNoSuffix::Int(x) => Numeric::Int(*x),
            value::NumberNoSuffix::UInt(x) => Numeric::UInt(*x),
            value::NumberNoSuffix::Float(Float64(x)) => Numeric::Float64(*x),
        }
    }
}

macro_rules! write_number {
    (
        $label:lifetime,
        $dst:ident,
        $variant:path,
        $numeric:ident,
        $buf:ident,
        $write_opts:path
    ) => {
        if let $variant(x) = $numeric {
            let sli =
                ::lexical_core::write_with_options::<_, { $crate::format::NUMBER_FORMAT }>(x, &mut $buf, &$write_opts);

            $dst.write_str(unsafe { ::core::str::from_utf8_unchecked(sli) })?;

            break $label;
        }
    };
}

fn write_number(
    mut dst: impl Write,
    number: Either<&value::Number, &value::NumberNoSuffix>,
    suffix_control: NumericSuffix,
) -> fmt::Result {
    let numeric = number.either_into::<Numeric>();
    let mut buf = [0x00u8; lexical_core::BUFFER_SIZE];

    'switch: {
        write_number!('switch, dst, Numeric::Int,      numeric, buf, format::WRITE_INTEGER_OPTS);
        write_number!('switch, dst, Numeric::UInt,     numeric, buf, format::WRITE_INTEGER_OPTS);
        write_number!('switch, dst, Numeric::LongInt,  numeric, buf, format::WRITE_INTEGER_OPTS);
        write_number!('switch, dst, Numeric::LongUInt, numeric, buf, format::WRITE_INTEGER_OPTS);
        write_number!('switch, dst, Numeric::Float32,  numeric, buf, format::WRITE_FLOAT_OPTS);
        write_number!('switch, dst, Numeric::Float64,  numeric, buf, format::WRITE_FLOAT_OPTS);
    }

    let Either::Left(number) = number else {
        return Ok(());
    };

    match number {
        value::Number::Int128(_) => dst.write_str("i128"),
        value::Number::UInt128(_) => dst.write_str("u128"),

        _ => match suffix_control <= NumericSuffix::IntegerOnly {
            true => match number {
                value::Number::Int8(_) => dst.write_str("i8"),
                value::Number::Int16(_) => dst.write_str("i16"),
                value::Number::Int32(_) => dst.write_str("i32"),
                value::Number::Int64(_) => dst.write_str("i64"),
                value::Number::UInt8(_) => dst.write_str("u8"),
                value::Number::UInt16(_) => dst.write_str("u16"),
                value::Number::UInt32(_) => dst.write_str("u32"),
                value::Number::UInt64(_) => dst.write_str("u64"),

                _ => match suffix_control <= NumericSuffix::Always {
                    true => match number {
                        value::Number::Float32(_) => dst.write_str("f32"),
                        value::Number::Float64(_) => dst.write_str("f64"),
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

fn write_nominal_path(
    mut dst: impl Write,
    path: NominalPath<'_>,
    enum_kind: bool,
    policy: NominalPathPolicy,
) -> fmt::Result {
    match path {
        NominalPath::Dual { name, parent } => match policy {
            NominalPathPolicy::Full => {
                dst.write_str(parent)?;
                dst.write_str("::")?;
                dst.write_str(name)
            }
            NominalPathPolicy::Named => dst.write_str(name),
            NominalPathPolicy::Minimal => match enum_kind {
                true => dst.write_str(name),
                false => dst.write_str("_"),
            },
        },
        NominalPath::Single { name } => match policy {
            NominalPathPolicy::Full => dst.write_str(name),
            NominalPathPolicy::Named => dst.write_str(name),
            NominalPathPolicy::Minimal => match enum_kind {
                true => dst.write_str(name),
                false => dst.write_str("_"),
            },
        },
        NominalPath::Unspecified => match enum_kind {
            true => panic!("missing enum name"),
            false => dst.write_str("_"),
        },
    }
}
