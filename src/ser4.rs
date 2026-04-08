use self::private::Token;
use crate::{
    format,
    value::{self, Float32, Float64},
};
use alloc::collections::VecDeque;
use core::fmt::{self, Write};

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
        TupleLike(Option<Nominal<'a>>),
        MapLike(Option<Nominal<'a>>),
        End,
    }

    pub enum Literal<'a> {
        Bool(bool),
        Char(char),
        Num(Number),
        Str(&'a str),
        Bytes(&'a [u8]),
    }

    pub enum Nominal<'a> {
        Unnamed,
        StemOnly { name: &'a str },
        FullNamed { name: &'a str, parent: &'a str },
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
            CompoundKind::Maybe => write!(dst, "?"),
            CompoundKind::Sequence => write!(dst, "["),
            CompoundKind::TupleLike(None) => write!(dst, "("),
            CompoundKind::TupleLike(Some(name)) => write!(dst, "{}(", name),
            CompoundKind::MapLikeLhs(None) | CompoundKind::MapLikeRhs(None) => write!(dst, "{{"),
            CompoundKind::MapLikeLhs(Some(name)) | CompoundKind::MapLikeRhs(Some(name)) => write!(dst, "{} {{", name),
        }
    }

    fn write_terminator(&self, dst: &mut impl Write) -> fmt::Result {
        match self {
            CompoundKind::Maybe => Ok(()),
            CompoundKind::Sequence => write!(dst, "]"),
            CompoundKind::TupleLike(_) => write!(dst, ")"),
            CompoundKind::MapLikeLhs(_) | CompoundKind::MapLikeRhs(_) => write!(dst, "}}"),
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
                    false => write!(dst, " ")?,
                }
                kind.write_indicator(dst)?;
                match kind.is_collection() {
                    true => write!(dst, "\n"),
                    false => Ok(()),
                }
            };

        let direct_write_entry = |dst: &mut W, depth: usize, kind: &mut CompoundKind, entry: &str| -> fmt::Result {
            match kind.single_line_child() {
                true => direct_write_indent(dst, depth)?,
                false => write!(dst, " ")?,
            }
            write!(dst, "{}", entry)?;
            match kind {
                CompoundKind::MapLikeLhs(name @ None) => {
                    *kind = CompoundKind::MapLikeRhs(name.take());
                    write!(dst, " =>")
                }
                CompoundKind::MapLikeLhs(name @ Some(_)) => {
                    *kind = CompoundKind::MapLikeRhs(name.take());
                    write!(dst, ":")
                }
                CompoundKind::MapLikeRhs(name) => {
                    *kind = CompoundKind::MapLikeLhs(name.take());
                    write!(dst, ",\n")
                }
                kind => match kind.is_collection() {
                    true => write!(dst, ",\n"),
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
                    None => write!(self.dst, "{};\n", entry)?,
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
                        write!(stringified, " ")?;
                    }
                    for _ in 0..entries_count.saturating_sub(1) {
                        write!(stringified, "{}, ", entries.next().unwrap())?;
                    }
                    if let Some(entry) = entries.next() {
                        write!(stringified, "{}", entry)?;
                    }
                    if kind.is_map_like() && entries_count > 0 {
                        write!(stringified, " ")?;
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
                            true => write!(dst, ",\n")?,
                            false => (),
                        },
                        None => write!(dst, ";\n")?,
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

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberSuffix {
    Explicit,
    IntegerOnly,
    #[default]
    LongIntegerOnly,
}

//==================================================================================================

trait Number {
    const SUFFIX: &str;
    const BITS: usize;
}

macro_rules! impl_number_for_primitive {
    ( $( $ty:ident ),* $(,)? ) => { $(
        impl Number for $ty {
            const SUFFIX: &str = stringify!($ty);
            const BITS: usize = size_of::<$ty>() * 8;
        }
    )* };
}

impl_number_for_primitive! {
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
             f32, f64,
}

macro_rules! write_concr_number {
    ( $label:lifetime, $dst:ident, $variant:path, $number:ident, $buf:ident, $write_opts:path ) => {
        if let $variant(x) = $number {
            let sli =
                ::lexical_core::write_with_options::<_, { $crate::format::NUMBER_FORMAT }>(x, &mut $buf, &$write_opts);

            $dst.write_str(unsafe { ::core::str::from_utf8_unchecked(sli) })?;

            break $label;
        }
    };
}

fn write_number(mut dst: impl Write, number: value::Number, suffix_control: NumberSuffix) -> fmt::Result {
    let mut buf = [0x00u8; lexical_core::BUFFER_SIZE];

    'switch: {
        write_concr_number!('switch, dst, value::Number::Int8, number, buf, format::WRITE_INTEGER_OPTS);
        write_concr_number!('switch, dst, value::Number::Int16, number, buf, format::WRITE_INTEGER_OPTS);
        write_concr_number!('switch, dst, value::Number::Int32, number, buf, format::WRITE_INTEGER_OPTS);
        write_concr_number!('switch, dst, value::Number::Int64, number, buf, format::WRITE_INTEGER_OPTS);
        // write_concrete_number!('switch, dst, value::Number::Int128, number, buf, format::WRITE_INTEGER_OPTS);
        write_concr_number!('switch, dst, value::Number::UInt8, number, buf, format::WRITE_INTEGER_OPTS);
        write_concr_number!('switch, dst, value::Number::UInt16, number, buf, format::WRITE_INTEGER_OPTS);
        write_concr_number!('switch, dst, value::Number::UInt32, number, buf, format::WRITE_INTEGER_OPTS);
        write_concr_number!('switch, dst, value::Number::UInt64, number, buf, format::WRITE_INTEGER_OPTS);
        // write_concrete_number!('switch, dst, value::Number::UInt128, number, buf, format::WRITE_INTEGER_OPTS);
        // value::Number::Float32(Float32(_)) => todo!(),
        // value::Number::Float64(Float64(_)) => todo!(),
    }

    todo!()
}

fn write_number_no_suffix(dst: impl Write, number: value::Number) -> fmt::Result {
    todo!()
}
