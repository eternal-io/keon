use alloc::collections::VecDeque;
use core::fmt::{self, Write};

//==================================================================================================

pub trait Serialize {
    fn seria_with(&self, ser: impl Serializer) -> fmt::Result;
}

pub trait Serializer {
    fn push(&mut self, token: Token) -> fmt::Result;
}

//==================================================================================================

#[doc(alias = "Serializer")]
pub struct Serria<W: Write> {
    dst: W,
    cfg: SerriaConfig,
    depth: usize,
    queue: VecDeque<Token>,
    stack: Vec<ContainerTerm>,
}

pub struct SerriaConfig {
    indent_width: usize,
    compact_width: usize,
}

pub enum Token {
    Stringified(String),
    Tuple,
    Seq,
    Map,
    NominalUnit(String),
    NominalTuple(String),
    NominalStruct(String),
    MaybeNone,
    MaybeSome,
    FatArrow,
    Colon,
    Comma,
    Close,
}

enum ContainerTerm {
    Compact {
        kind: ContainerKind,
        acc_width: usize,
        queue_index: usize,
    },
    Expanded {
        kind: ContainerKind,
    },
}

enum ContainerKind {
    Seq,
    MapLike,
    TupleLike,
}

impl<W: Write> Serializer for &mut Serria<W> {
    fn push(&mut self, token: Token) -> fmt::Result {
        let dst = &mut self.dst;
        let write_indent =
            |dst: &mut W, depth: usize| (0..self.cfg.indent_width * depth).try_for_each(|_| dst.write_str(" "));

        let enter_container = |stack: &mut Vec<ContainerTerm>, queue: &mut VecDeque<Token>, token: Token| {
            let kind = match &token {
                Token::Seq => ContainerKind::Seq,
                Token::Map | Token::NominalStruct(_) => ContainerKind::MapLike,
                Token::Tuple | Token::NominalTuple(_) => ContainerKind::TupleLike,
                _ => unreachable!(),
            };
            let acc_width = stack
                .last()
                .map(|term| match term {
                    ContainerTerm::Compact { acc_width, .. } => *acc_width,
                    ContainerTerm::Expanded { .. } => 0,
                })
                .unwrap_or(0)
                + match &token {
                    Token::Tuple => 2,
                    Token::Seq => 2,
                    Token::Map => 4,
                    Token::NominalTuple(name) => name.chars().count() + 2,
                    Token::NominalStruct(name) => name.chars().count() + 3,
                    _ => unreachable!(),
                };
            stack.push(ContainerTerm::Compact {
                kind,
                acc_width,
                queue_index: queue.len(),
            });
            queue.push_back(token);
        };

        debug_assert!(self.depth <= self.stack.len());

        if self.depth == self.stack.len() {
            match token {
                Token::Tuple | Token::Seq | Token::Map | Token::NominalTuple(_) | Token::NominalStruct(_) => {
                    enter_container(&mut self.stack, &mut self.queue, token)
                }

                Token::Stringified(s) | Token::NominalUnit(s) => {
                    write_indent(dst, self.depth)?;
                    dst.write_str(&s)?;
                }

                Token::MaybeNone => dst.write_str("?")?,
                Token::MaybeSome => dst.write_str("? ")?,
                Token::FatArrow => dst.write_str(" => ")?,
                Token::Colon => dst.write_str(": ")?,
                Token::Comma => dst.write_str(",\n")?,
                Token::Close => {
                    self.depth -= 1;
                    match self.stack.pop().unwrap() {
                        ContainerTerm::Compact { .. } => panic!(),
                        ContainerTerm::Expanded { kind } => {
                            write_indent(dst, self.depth)?;
                            match kind {
                                ContainerKind::Seq => dst.write_str(")")?,
                                ContainerKind::MapLike => dst.write_str("}")?,
                                ContainerKind::TupleLike => dst.write_str("]")?,
                            }
                        }
                    }
                }
            }
        } else {
            match token {
                Token::Tuple | Token::Seq | Token::Map | Token::NominalTuple(_) | Token::NominalStruct(_) => {
                    enter_container(&mut self.stack, &mut self.queue, token);
                    return Ok(());
                }

                Token::Close => match self.stack.pop().unwrap() {
                    ContainerTerm::Expanded { .. } => panic!(),
                    ContainerTerm::Compact { queue_index, .. } => {
                        let mut entries = self.queue.drain(queue_index..);
                        let mut stringified = String::new();
                        let indicator = entries.next().unwrap();

                        todo!()
                    }
                },

                _ => self.queue.push_back(token),
            }
        }

        Ok(())
    }
}
