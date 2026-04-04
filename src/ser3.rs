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
    Compact { acc_width: usize, queue_index: usize },
    Expanded { kind: ContainerKind },
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
            stack.push(ContainerTerm::Compact {
                acc_width: stack
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
                    },
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
                    let ContainerTerm::Expanded { kind } = self.stack.pop().unwrap() else {
                        panic!()
                    };

                    write_indent(dst, self.depth)?;
                    match kind {
                        ContainerKind::Seq => dst.write_str(")")?,
                        ContainerKind::MapLike => dst.write_str("}")?,
                        ContainerKind::TupleLike => dst.write_str("]")?,
                    }
                }
            }
        } else {
            match token {
                Token::Tuple | Token::Seq | Token::Map | Token::NominalTuple(_) | Token::NominalStruct(_) => {
                    enter_container(&mut self.stack, &mut self.queue, token);
                    return Ok(());
                }

                Token::Close => {
                    let ContainerTerm::Compact { acc_width, queue_index } = self.stack.pop().unwrap() else {
                        panic!()
                    };

                    let mut entries = self.queue.drain(queue_index..);
                    let mut stringified = String::new();
                    let indicator = entries.next().unwrap();

                    match indicator {
                        Token::Tuple => todo!(),
                        Token::Seq => todo!(),
                        Token::Map => todo!(),
                        Token::NominalTuple(_) => todo!(),
                        Token::NominalStruct(_) => todo!(),
                        _ => unreachable!(),
                    }

                    todo!()
                }

                token => {
                    let ContainerTerm::Compact { acc_width, queue_index } = self.stack.last_mut().unwrap() else {
                        panic!()
                    };

                    *acc_width += match &token {
                        Token::Stringified(s) | Token::NominalUnit(s) => s.len(),
                        Token::MaybeNone => 1,
                        Token::MaybeSome => 2,
                        Token::FatArrow => 4,
                        Token::Colon => 2,
                        Token::Comma => 2,
                        _ => unreachable!(),
                    };

                    self.queue.push_back(token);

                    if *acc_width > self.cfg.compact_width {
                        todo!()
                    }
                }
            }
        }

        Ok(())
    }
}
