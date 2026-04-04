use alloc::collections::VecDeque;
use core::fmt::{self, Write};

//==================================================================================================

pub trait Serialize {
    fn seria_with(&self, ser: impl Serializer) -> fmt::Result;
}

pub trait Serializer {
    fn push(&mut self, token: Token) -> fmt::Result;
}

#[doc(hidden)]
pub enum Token {
    Stringified(String),
    Maybe,
    Sequence,
    TupleLike(Option<String>),
    MapLike(Option<String>),
    End,
}

//==================================================================================================

#[doc(alias = "Serializer")]
pub struct FastSerria {}

//==================================================================================================

#[doc(alias = "Serializer")]
pub struct Serria<W: Write> {
    dst: W,
    cfg: SerriaConfig,
    compound_stack: Vec<Compound>,
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
        let cfg = &self.cfg;
        let direct_write_indent = |dst: &mut W, depth: usize| -> fmt::Result {
            (0..cfg.indent_width as usize * depth).try_for_each(|_| dst.write_str(" "))
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

        let direct_write_entry = |dst: &mut W, depth: usize, kind: &mut CompoundKind, entry: String| -> fmt::Result {
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
            |dst: &mut W, compound_stack: &mut Vec<Compound>, inline_entries: &mut VecDeque<String>| -> fmt::Result {
                let compounds_count = compound_stack.len();
                let mut single_line_child;
                for i in 0..compounds_count {
                    if let Compound::Expanded { .. } = compound_stack[i] {
                        continue;
                    }

                    single_line_child = i
                        .checked_sub(1)
                        .map(|i| compound_stack[i].kind().single_line_child())
                        .unwrap_or(false);
                    let range_end = match compound_stack.get(i + 1) {
                        Some(Compound::Compact {
                            inline_entries_index, ..
                        }) => *inline_entries_index,
                        _ => compounds_count,
                    };
                    let Compound::Compact {
                        ref mut kind,
                        inline_entries_index: range_start,
                        ..
                    } = compound_stack[i]
                    else {
                        unreachable!()
                    };

                    direct_write_indicator(dst, i, single_line_child, kind)?;
                    inline_entries
                        .drain(..range_end - range_start)
                        .try_for_each(|entry| direct_write_entry(dst, i + 1, kind, entry))?;

                    compound_stack[i] = Compound::Expanded { kind: kind.take() };
                }
                Ok(())
            };

        match token {
            Token::Stringified(entry) => {
                if self.compound_stack.is_empty() {
                    return write!(self.dst, "{};\n", entry);
                }
                match self.compound_stack.last().unwrap() {
                    Compound::Compact {
                        kind,
                        force_compact,
                        inline_entries_index,
                    } => {
                        self.inline_entries.push_back(entry);
                        if !*force_compact
                            && if kind.is_map_like() {
                                self.inline_entries.len() - *inline_entries_index
                                    > self.cfg.map_like_inline_entries as usize /* conditionally compact `{}` */
                            } else {
                                false /* unconditionally compact `?`, `[]` and `()` while pushing stringified */
                            }
                        {
                            break_and_flush(dst, &mut self.compound_stack, &mut self.inline_entries)?;
                        }
                    }
                    Compound::Expanded { .. } => {
                        let depth = self.compound_stack.len();
                        let Compound::Expanded { ref mut kind } = self.compound_stack.last_mut().unwrap() else {
                            unreachable!()
                        };
                        direct_write_entry(dst, depth, kind, entry)?;
                    }
                }
            }

            Token::End => match self.compound_stack.pop().unwrap() {
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

                    return self.push(Token::Stringified(stringified));
                }
                Compound::Expanded { kind } => {
                    direct_write_indent(dst, self.compound_stack.len())?;

                    kind.write_terminator(dst)?;

                    match self.compound_stack.last() {
                        Some(comp) => match comp.kind().is_collection() {
                            true => write!(dst, ",\n")?,
                            false => (),
                        },
                        None => write!(dst, ";\n")?,
                    }
                }
            },

            token => {
                match token {
                    Token::Maybe => CompoundKind::Maybe,
                    Token::Sequence => CompoundKind::Sequence,
                    Token::TupleLike(name) => CompoundKind::TupleLike(name),
                    Token::MapLike(name) => CompoundKind::MapLikeLhs(name),
                    _ => unreachable!(),
                };
                todo!()
            }
        }

        todo!()
    }
}
