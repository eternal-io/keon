use self::error::*;
use alloc::collections::{vec_deque, VecDeque};
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
    Compressed,
    Expanded,
}

#[derive(Debug, Clone)]
enum CompoundKind {
    Maybe,
    Tuple,                 // (T, U, ...)
    Seq,                   // [T, T, ...]
    Map,                   // { K => V }
    MapPair,               //   K
    MapPairRhs,            //     => V
    NominalUnit(String),   // Name
    NominalTuple(String),  // Name(T)
    NominalStruct(String), // Name { field: T }
    StructField,           //        field
    StructFieldRhs,        //             : T
}

struct CompoundTerm {
    ctrl: LayoutControl,
    kind: CompoundKind,

    /// Meaningless if `ctrl == Expanded`.
    queue_index: usize,

    /// Must be zero if `ctrl == Expanded`.
    cumulative_width: usize,
}

impl<W: Write> Serria<W> {
    pub fn seria<T: Seriable>(&mut self, value: &T) -> fmt::Result {
        todo!()
    }

    fn begin(&mut self) -> SeriaResult<SerriaGuard<'_, W>> {
        self.flush_error()?;
        self.queue.make_contiguous();
        Ok(SerriaGuard { serria: self })
    }

    #[inline]
    fn flush_error(&mut self) -> SeriaResult {
        match self.deferred_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    #[inline]
    fn flush_layout(&mut self) -> SeriaResult {
        if self.stack.last().unwrap().cumulative_width <= self.cfg.max_width {
            return Ok(());
        }

        if self.stack.len() > 1 {
            for i in 0..self.stack.len() - 1 {
                let comp = &self.stack[i];
                let comp_next = &self.stack[i + 1];

                if comp.ctrl == LayoutControl::Expanded {
                    continue;
                }

                let range_start = comp.queue_index;
                let range_end = comp_next.queue_index;

                self.expand(i, range_end - range_start)?;
            }
        }

        self.expand(self.stack.len() - 1, self.queue.len())?;

        Ok(())
    }

    #[inline]
    fn expand(&mut self, depth: usize, count: usize) -> SeriaResult {
        let dst = &mut self.dst;
        let indent_width = self.cfg.indent_width;
        let write_indent = |dst: &mut W, depth: usize| -> SeriaResult {
            (0..indent_width * depth)
                .try_for_each(|_| dst.write_str(" "))
                .map_err(Into::into)
        };

        let comp = &mut self.stack[depth];
        let mut entries = self.queue.drain(..count);

        debug_assert!(comp.ctrl == LayoutControl::Compressed);

        comp.ctrl = LayoutControl::Expanded;
        comp.cumulative_width = 0;

        write_indent(dst, depth)?;

        match &comp.kind {
            CompoundKind::Seq => writeln!(dst, "[")?,
            CompoundKind::Tuple => writeln!(dst, "(")?,
            CompoundKind::Map => writeln!(dst, "{{")?,

            CompoundKind::NominalTuple(name) => writeln!(dst, "{}(", name)?,
            CompoundKind::NominalStruct(name) => writeln!(dst, "{} {{", name)?,

            CompoundKind::NominalUnit(name) => {
                write!(dst, "{}", name)?;

                debug_assert!(entries.next().is_none(), "'nominal unit' accepts no entry");
                return Ok(());
            }

            CompoundKind::Maybe => {
                write!(dst, "? ")?; // It must be some if expand has been triggered.
                if let Some(entry) = entries.next() {
                    write!(dst, "{}", entry)?;
                }
                debug_assert!(entries.next().is_none(), "'maybe value' accepts at most one entry");
                return Ok(());
            }

            pair => {
                match entries.next() {
                    None => debug_assert!(
                        !matches!(pair, CompoundKind::StructField),
                        "'struct field' left-hand side must be literal (identifier)"
                    ),
                    Some(entry) => match pair {
                        CompoundKind::MapPair => write!(dst, "{} => ", entry)?,
                        CompoundKind::MapPairRhs => (),
                        CompoundKind::StructField => write!(dst, "{}: ", entry)?,
                        CompoundKind::StructFieldRhs => (),
                        _ => unreachable!(),
                    },
                }
                debug_assert!(
                    entries.next().is_none(),
                    "'pair kind' accepts at most one entry while expanding"
                );
                return Ok(());
            }
        }

        for entry in entries {
            write_indent(dst, depth + 1)?;
            writeln!(dst, "{},", entry)?;
        }

        Ok(())
    }

    fn compress(&mut self) -> SeriaResult {
        let Some(comp) = self.stack.last() else {
            return Ok(());
        };

        debug_assert!(comp.ctrl == LayoutControl::Compressed);

        let mut entries = self.queue.drain(comp.queue_index..).peekable();
        let mut stringified = String::new();

        'comma_joined: {
            let non_empty = entries.peek().is_some();

            match &comp.kind {
                CompoundKind::Seq => write!(stringified, "[")?,
                CompoundKind::Tuple => write!(stringified, "(")?,
                CompoundKind::Map => {
                    write!(stringified, "{{")?;
                    if non_empty {
                        write!(stringified, " ")?;
                    }
                }

                CompoundKind::NominalTuple(name) => write!(stringified, "{}(", name)?,
                CompoundKind::NominalStruct(name) => {
                    write!(stringified, "{} {{", name)?;
                    if non_empty {
                        write!(stringified, " ")?;
                    }
                }
                CompoundKind::NominalUnit(name) => {
                    write!(stringified, "{}", name)?;

                    debug_assert!(entries.next().is_none(), "'nominal unit' accepts no entry");
                    break 'comma_joined;
                }

                CompoundKind::Maybe => {
                    write!(stringified, "?")?;
                    if let Some(entry) = entries.next() {
                        write!(stringified, " {}", entry)?
                    }
                    debug_assert!(entries.next().is_none(), "'maybe value' accepts at most one entry");
                    break 'comma_joined;
                }

                CompoundKind::MapPair => unreachable!("'map pair' must not missing value"),
                CompoundKind::MapPairRhs => {
                    let k = entries.next().expect("map pair key");
                    let v = entries.next().expect("map pair value");
                    write!(stringified, "{} => {}", k, v)?;

                    debug_assert!(entries.next().is_none(), "'map pair' accepts exactly two entries");
                    break 'comma_joined;
                }

                CompoundKind::StructField => unreachable!("'struct field' must not missing value"),
                CompoundKind::StructFieldRhs => {
                    let f = entries.next().expect("struct field key");
                    let v = entries.next().expect("struct field value");
                    write!(stringified, "{}: {}", f, v)?;

                    debug_assert!(entries.next().is_none(), "'struct field' accepts exactly two entries");
                    break 'comma_joined;
                }
            }

            while entries.peek().is_some() {
                let entry = entries.next().unwrap();
                write!(stringified, "{}, ", entry)?;
            }
            if let Some(entry) = entries.next() {
                write!(stringified, "{}", entry)?;
            }

            match &comp.kind {
                CompoundKind::Seq => write!(stringified, "]")?,
                CompoundKind::Tuple | CompoundKind::NominalTuple(_) => write!(stringified, ")")?,
                CompoundKind::Map | CompoundKind::NominalStruct(_) => {
                    if non_empty {
                        write!(stringified, " ")?;
                    }
                    write!(stringified, "}}")?;
                }
                _ => unreachable!(),
            }
        }

        drop(entries);
        self.queue.push_back(stringified);

        Ok(())
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
                CompoundKind::NominalUnit(_) => return Err(SeriaError::TooManyEntries),
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
                CompoundKind::StructField => match entries_count {
                    0 => container.cumulative_width += literal.len() + 1,
                    1 => container.cumulative_width += 1 + literal.len(),
                    _ => return Err(SeriaError::TooManyEntries),
                },

                CompoundKind::MapPairRhs => todo!(),
                CompoundKind::StructFieldRhs => todo!(),
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
