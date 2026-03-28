use self::error::*;
use alloc::collections::VecDeque;
use core::{
    fmt::{self, Write},
    ops::{Deref, DerefMut},
};

pub mod error;

pub fn fast_seria<T: Seriable>(value: T) -> String {
    todo!()
}

//==================================================================================================

#[doc(alias = "Serialize")]
pub trait Seriable {
    fn seria_with<W: Write>(&self, ser: &mut Serria<W>) -> SeriaResult;
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
    stack: Vec<CompoundTerm>,
    queue: VecDeque<String>,
    deferred_err: Option<SeriaError>,
}

pub struct SerriaConfig {
    max_width: usize,
    indent_width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutControl {
    Collapsed,
    Expanded,
}

enum CompoundKind {
    Maybe,
    Tuple,                 // (T, U, ...)
    Seq,                   // [T, T, ...]
    Map,                   // { K => V }
    MapPair,               //   K
    MapPairRhs,            //     => V
    NominalUnit,           // Name
    NominalTuple(String),  // Name(T)
    NominalStruct(String), // Name { field: T }
    StructPair,            //        field
    StructPairRhs,         //             : T
}

struct CompoundTerm {
    ctrl: LayoutControl,
    kind: CompoundKind,
    queue_index: usize,
    cumulative_width: usize,
}

impl<W: Write> Serria<W> {
    pub fn start(&mut self) -> SeriaResult<SerriaGuard<'_, W>> {
        self.flush_error()?;
        self.queue.make_contiguous();
        Ok(SerriaGuard { serria: self })
    }

    pub fn finish(mut self) -> SeriaResult {
        self.flush_error()
    }

    fn flush_error(&mut self) -> SeriaResult {
        match self.deferred_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn flush_layout(&mut self) -> SeriaResult {
        if self.stack.last().unwrap().cumulative_width <= self.cfg.max_width {
            return Ok(());
        }

        if self.stack.len() > 1 {
            for i in 0..self.stack.len() - 1 {
                let [comp, comp_next] = self.stack.get_disjoint_mut([i, i + 1]).unwrap();

                let range_start = comp.queue_index;
                let range_end = comp_next.queue_index;

                self.expand(i, range_end - range_start)?;
            }
        }

        if !self.stack.is_empty() {
            self.expand(self.stack.len() - 1, self.queue.len())?;
        }

        Ok(())
    }

    fn write_indent(&mut self) -> SeriaResult {
        (0..self.cfg.indent_width * self.stack.len())
            .try_for_each(|_| self.dst.write_str(" "))
            .map_err(Into::into)
    }

    fn write_indent_at(&mut self, depth: usize) -> SeriaResult {
        (0..self.cfg.indent_width * depth)
            .try_for_each(|_| self.dst.write_str(" "))
            .map_err(Into::into)
    }

    #[inline]
    fn expand(&mut self, index: usize, count: usize) -> SeriaResult {
        let dst = &mut self.dst;
        let indent_width = self.cfg.indent_width;
        let mut write_indent = |depth: usize| -> SeriaResult {
            (0..indent_width * depth)
                .try_for_each(|_| dst.write_str(" "))
                .map_err(Into::into)
        };

        let comp = &mut self.stack[index];
        let mut entries = self.queue.drain(..count);

        write_indent(index)?;

        match comp.kind {
            CompoundKind::Maybe => match entries.next() {
                None => Err(SeriaError::TooFewEntries),
                Some(_) => todo!(),
            },
            CompoundKind::Tuple => todo!(),
            CompoundKind::Seq => todo!(),
            CompoundKind::Map => todo!(),
            CompoundKind::MapPair => todo!(),
            CompoundKind::MapPairRhs => Ok(()),
            CompoundKind::NominalUnit => todo!(),
            CompoundKind::NominalTuple(_) => todo!(),
            CompoundKind::NominalStruct(_) => todo!(),
            CompoundKind::StructPair => todo!(),
            CompoundKind::StructPairRhs => Ok(()),
        }
    }
}

//------------------------------------------------------------------------------

struct SerriaGuard<'w, W: Write> {
    serria: &'w mut Serria<W>,
}

impl<W: Write> SerriaGuard<'_, W> {
    fn push(&mut self, literal: String) -> SeriaResult {
        self.flush_error()?;

        let queue_len = self.queue.len();
        if let Some(container) = self.stack.last_mut() {
            let entries_count = queue_len.strict_sub(container.queue_index);

            match container.kind {
                CompoundKind::Maybe => match entries_count {
                    0 => container.cumulative_width += 1 + literal.len(),
                    _ => return Err(SeriaError::TooManyEntries),
                },
                CompoundKind::NominalUnit => return Err(SeriaError::TooManyEntries),
                CompoundKind::Seq | CompoundKind::NominalTuple(_) | CompoundKind::Tuple => match entries_count {
                    0 => container.cumulative_width += literal.len(),
                    _ => container.cumulative_width += 2 + literal.len(),
                },
                CompoundKind::Map | CompoundKind::NominalStruct(_) => match entries_count {
                    0 => container.cumulative_width += 1 + literal.len() + 1,
                    _ => container.cumulative_width += 2 + literal.len(),
                },
                CompoundKind::MapPair => match entries_count {
                    0 => container.cumulative_width += literal.len(),
                    1 => container.cumulative_width += 1 + 2 + 1 + literal.len(),
                    _ => return Err(SeriaError::TooManyEntries),
                },
                CompoundKind::StructPair => match entries_count {
                    0 => container.cumulative_width += literal.len() + 1,
                    1 => container.cumulative_width += 1 + literal.len(),
                    _ => return Err(SeriaError::TooManyEntries),
                },

                CompoundKind::MapPairRhs => todo!(),
                CompoundKind::StructPairRhs => todo!(),
            }

            if container.cumulative_width > self.queue.len() {}
        }

        self.queue.push_back(literal);

        Ok(())
    }

    fn enter(&mut self, kind: CompoundKind) -> SeriaResult<SerriaGuard<'_, W>> {
        self.flush_error()?;

        todo!()
    }
}

impl<W: Write> Drop for SerriaGuard<'_, W> {
    fn drop(&mut self) {
        fn foo() {}

        //------------------------------------------------------------------------------

        let queue_len = self.queue.len();
        let deferred_err = match self.stack.pop() {
            None => {
                let entries_count = queue_len;
                match entries_count {
                    1 => {
                        let entry = self.queue.pop_back().unwrap();
                        self.dst.write_str(&entry).err().map(Into::into)
                    }
                    0 => Some(SeriaError::TooFewEntries),
                    _ => Some(SeriaError::TooManyEntries),
                }
            }
            Some(container) => {
                todo!()

                // match container.ctrl {
                //     LayoutControl::Compact => {
                //         let entries_count = queue_len.strict_sub(container.queue_index);
                //         match container.kind {
                //             ContainerKind::Maybe => match entries_count {
                //                 0 => {
                //                     self.queue.push(format!("?"));
                //                     None
                //                 }
                //                 1 => {
                //                     let entry = self.queue.pop().unwrap();
                //                     self.queue.push(format!("? {}", entry));
                //                     None
                //                 }
                //                 _ => Some(SeriaError::TooManyEntries),
                //             },

                //             ContainerKind::Tuple => {
                //                 let mut stringified = String::new();
                //                 {
                //                     let mut entries = self.queue.drain(container.queue_index..).peekable();

                //                     stringified.push_str("(");

                //                     while entries.peek().is_some() {
                //                         stringified.push_str(&entries.next().unwrap());
                //                         stringified.push_str(", ");
                //                     }
                //                     if let Some(entry) = entries.next() {
                //                         stringified.push_str(&entry);
                //                     }

                //                     stringified.push_str(")");
                //                 }
                //                 self.queue.push(stringified);

                //                 None
                //             }

                //             ContainerKind::Seq => todo!(),
                //             ContainerKind::Map => todo!(),

                //             ContainerKind::MapPair => match entries_count {
                //                 2 => {
                //                     let v = self.queue.pop().unwrap();
                //                     let k = self.queue.pop().unwrap();
                //                     self.queue.push(format!("{} => {}", k, v));
                //                     None
                //                 }
                //                 1 | 0 => Some(SeriaError::TooFewEntries),
                //                 _ => Some(SeriaError::TooManyEntries),
                //             },

                //             ContainerKind::NominalUnit => todo!(),
                //             ContainerKind::NominalTuple(_) => todo!(),
                //             ContainerKind::NominalStruct(_) => todo!(),

                //             ContainerKind::StructPair => match entries_count {
                //                 2 => {
                //                     let v = self.queue.pop().unwrap();
                //                     let k = self.queue.pop().unwrap();
                //                     self.queue.push(format!("{}: {}", k, v));
                //                     None
                //                 }
                //                 1 | 0 => Some(SeriaError::TooFewEntries),
                //                 _ => Some(SeriaError::TooManyEntries),
                //             },
                //         }
                //     }

                //     LayoutControl::Expanded => match container.kind {
                //         ContainerKind::Maybe => None,

                //         ContainerKind::Tuple => writeln!(self.dst, ",")
                //             .err()
                //             .map(Into::into)
                //             .or_else(|| self.write_indent().err())
                //             .or_else(|| self.dst.write_str(")").err().map(Into::into)),

                //         ContainerKind::Seq | ContainerKind::NominalTuple(_) => writeln!(self.dst, ",")
                //             .err()
                //             .map(Into::into)
                //             .or_else(|| self.write_indent().err())
                //             .or_else(|| self.dst.write_str("]").err().map(Into::into)),

                //         ContainerKind::Map | ContainerKind::NominalStruct(_) => writeln!(self.dst, ",")
                //             .err()
                //             .map(Into::into)
                //             .or_else(|| self.write_indent().err())
                //             .or_else(|| self.dst.write_str("}").err().map(Into::into)),

                //         ContainerKind::MapPair => None,

                //         ContainerKind::NominalUnit => None,

                //         ContainerKind::StructPair => None,
                //     },
                // }
            }
        };

        self.deferred_err = self.deferred_err.or(deferred_err);
    }
}

impl<W: Write> Deref for SerriaGuard<'_, W> {
    type Target = Serria<W>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.serria
    }
}

impl<W: Write> DerefMut for SerriaGuard<'_, W> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.serria
    }
}

//------------------------------------------------------------------------------

//------------------------------------------------------------------------------
