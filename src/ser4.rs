use alloc::collections::{vec_deque, VecDeque};
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

    fn child_leading_indent(&self) -> bool {
        match self.kind() {
            CompoundKind::Maybe => false,
            CompoundKind::Sequence => true,
            CompoundKind::TupleLike(_) => true,
            CompoundKind::MapLikeLhs(_) => true,
            CompoundKind::MapLikeRhs(_) => false,
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

// impl CompoundKind {
//     fn take(&mut self) -> Self {
//         match self {
//             CompoundKind::Maybe => CompoundKind::Maybe,
//             CompoundKind::Sequence => CompoundKind::Sequence,
//             CompoundKind::TupleLike(name) => CompoundKind::TupleLike(name.take()),
//             CompoundKind::MapLikeLhs(name) => CompoundKind::MapLikeLhs(name.take()),
//             CompoundKind::MapLikeRhs(name) => CompoundKind::MapLikeRhs(name.take()),
//         }
//     }
// }

impl<W: Write> Serializer for &mut Serria<W> {
    fn push(&mut self, token: Token) -> fmt::Result {
        let dst = &mut self.dst;
        let cfg = &self.cfg;
        let write_indent =
            |dst: &mut W, depth: usize| (0..cfg.indent_width as usize * depth).try_for_each(|_| dst.write_str(" "));

        // TODO: write_indicator, write_entry

        let perform_expand = |dst: &mut W,
                              depth: usize,
                              leading_indent: bool,
                              kind: &mut CompoundKind,
                              entries: vec_deque::Drain<'_, String>|
         -> fmt::Result {
            if leading_indent {
                write_indent(dst, depth)?;
            }

            for (i, entry) in entries.enumerate() {}

            Ok(())
        };

        match token {
            Token::Stringified(s) => {
                if self.compound_stack.is_empty() {
                    return writeln!(self.dst, "{};", s);
                }

                match self.compound_stack.last().unwrap() {
                    Compound::Expanded { .. } => todo!(),

                    Compound::Compact {
                        kind,
                        force_compact,
                        inline_entries_index,
                    } => {
                        self.inline_entries.push_back(s);

                        if !force_compact
                            && if let CompoundKind::MapLikeLhs(_) | CompoundKind::MapLikeRhs(_) = kind {
                                /* conditionally compact `{}` */
                                self.inline_entries.len() - *inline_entries_index
                                    > self.cfg.map_like_inline_entries as usize
                            } else {
                                /* unconditionally compact `?`, `[]` and `()` while pushing stringified */
                                false
                            }
                        {
                            /* perform expand */
                            for i in 0..self.compound_stack.len() - 1 {
                                if let Compound::Expanded { .. } = self.compound_stack[i] {
                                    continue;
                                }

                                let leading_indent = i
                                    .checked_sub(1)
                                    .map(|i| self.compound_stack[i].child_leading_indent())
                                    .unwrap_or(false);

                                let Compound::Compact {
                                    inline_entries_index: range_end,
                                    ..
                                } = self.compound_stack[i + 1]
                                else {
                                    unreachable!()
                                };

                                let Compound::Compact {
                                    ref mut kind,
                                    inline_entries_index: range_start,
                                    ..
                                } = self.compound_stack[i]
                                else {
                                    unreachable!()
                                };

                                perform_expand(
                                    dst,
                                    i,
                                    leading_indent,
                                    kind,
                                    self.inline_entries.drain(..range_end - range_start),
                                )?;
                            }
                        }
                    }
                }
            }

            Token::End => {
                todo!();

                if self.compound_stack.is_empty() {
                    writeln!(dst, ";")?;
                }
            }

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
