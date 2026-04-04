use self::{error::Reason, private::Sealed};
use alloc::collections::{vec_deque, VecDeque};
use core::{
    fmt::{self, Write},
    ops::{Deref, DerefMut},
};

pub mod error;

mod private {
    pub trait Sealed {}
}

//==================================================================================================

pub fn fast_seria<T: Serialize>(value: T) -> String {
    todo!()
}

//==================================================================================================

pub trait Serialize {
    fn seria_with<'w, W: Write>(&self, ser: impl Serializer<'w>) -> fmt::Result;
}

pub trait Serializer<'w>: Sized + Sealed {
    type SerializerVisitor: SerializerVisitor<'w>;

    fn begin(&'w mut self) -> Result<Self::SerializerVisitor, fmt::Error>;
}

#[doc(hidden)]
pub trait SerializerVisitor<'w>: Sized + Sealed {
    fn style(&self) -> ! {
        unimplemented!()
    }

    fn enter(&'w mut self, kind: CompoundKind) -> Result<Self, fmt::Error>;

    fn push(&mut self, entry: String) -> fmt::Result;
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
    deferred_err: bool,
}

pub struct SerriaConfig {
    max_width: usize,
    indent_width: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutControl {
    Compact,
    Expanded,
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum CompoundKind {
    Maybe,
    Tuple,                 // (T, U, ...)
    Seq,                   // [T, T, ...]
    Map,                   // { K => V }
    MapPair,               //   K => V
    NominalUnit(String),   // Name
    NominalTuple(String),  // Name(T)
    NominalStruct(String), // Name { field: T }
    StructField,           //        field: T
}

struct CompoundTerm {
    ctrl: LayoutControl,
    kind: CompoundKind,

    /// Meaningless if `ctrl == Expanded`.
    queue_index: usize,

    /// Must be zero if `ctrl == Expanded`.
    cumulative_width: usize,
}

impl<W: Write> Sealed for &mut Serria<W> {}

impl<'w, W: Write> Serializer<'w> for &'w mut Serria<W> {
    type SerializerVisitor = SerriaVisitor<'w, W>;

    fn begin(&'w mut self) -> Result<Self::SerializerVisitor, fmt::Error> {
        self.flush_error()?;
        self.queue.make_contiguous();
        Ok(SerriaVisitor { ser: self })
    }
}

impl<W: Write> Sealed for SerriaVisitor<'_, W> {}

impl<'w, W: Write> SerializerVisitor<'w> for SerriaVisitor<'w, W> {
    fn enter(&'w mut self, kind: CompoundKind) -> Result<Self, fmt::Error> {
        self.flush_error()?;

        if let Some(comp) = self.stack.last() {
            if comp.ctrl == LayoutControl::Expanded {
                match comp.kind {
                    CompoundKind::MapPair => write!(self.dst, " => ")?,
                    CompoundKind::StructField => write!(self.dst, ": ")?,
                    _ => (),
                }
            }
        }

        let cumulative_width = self.stack.last().map(|comp| comp.cumulative_width).unwrap_or(0)
            + match &kind {
                CompoundKind::Maybe | CompoundKind::Tuple | CompoundKind::Seq | CompoundKind::Map => 1,
                CompoundKind::NominalUnit(name) => name.chars().count(),
                CompoundKind::NominalTuple(name) => name.chars().count() + 1,
                CompoundKind::NominalStruct(name) => name.chars().count() + 2,
                CompoundKind::MapPair | CompoundKind::StructField => 0,
            };

        let next_comp = CompoundTerm {
            ctrl: LayoutControl::Compact,
            kind,
            queue_index: self.queue.len(),
            cumulative_width,
        };

        self.stack.push(next_comp);

        Ok(SerriaVisitor { ser: self.ser })
    }

    fn push(&mut self, entry: String) -> fmt::Result {
        // self.flush_error()?;

        // let dst = &mut self.dst;
        // let write_indent = |dst: &mut W, depth| (0..self.cfg.indent_width * depth).try_for_each(|_| dst.write_str(" "));

        // let Some(comp) = self.stack.last_mut() else {
        //     todo!();
        // };

        // if comp.ctrl == LayoutControl::Expanded {
        //     match comp.kind {
        //         CompoundKind::MapPair => write!(dst, " => ")?,
        //         CompoundKind::StructField => write!(dst, ": ")?,
        //         _ => (),
        //     }

        //     write_indent(dst, self.stack.len())?;
        // }

        todo!()
    }
}

impl<W: Write> Serria<W> {
    pub fn seria<T: Serialize>(&mut self, value: &T) -> fmt::Result {
        todo!()
    }

    #[inline]
    fn flush_error(&mut self) -> fmt::Result {
        match self.deferred_err {
            true => Err(fmt::Error),
            false => Ok(()),
        }
    }

    #[inline]
    fn flush_layout(&mut self) -> fmt::Result {
        assert!(!self.stack.is_empty());

        for i in 0..self.stack.len() - 1 {
            let comp = &self.stack[i];
            let comp_next = &self.stack[i + 1];

            if comp.ctrl == LayoutControl::Expanded {
                continue;
            }

            let range_start = comp.queue_index;
            let range_end = comp_next.queue_index;

            self.perform_expand(i, range_end - range_start)?;

            if self.stack.last().unwrap().cumulative_width <= self.cfg.max_width {
                return Ok(());
            }
        }

        self.perform_expand(self.stack.len() - 1, self.queue.len())?;

        Ok(())
    }

    #[inline]
    fn perform_expand(&mut self, depth: usize, entries_count: usize) -> fmt::Result {
        let dst = &mut self.dst;
        let write_indent = |dst: &mut W, depth| (0..self.cfg.indent_width * depth).try_for_each(|_| dst.write_str(" "));

        write_indent(dst, depth)?;

        let comp = &mut self.stack[depth];
        debug_assert!(comp.ctrl == LayoutControl::Compact);
        comp.ctrl = LayoutControl::Expanded;

        'comma_joined: {
            let mut entries = self.queue.drain(..entries_count);
            match &comp.kind {
                CompoundKind::Seq => writeln!(dst, "[")?,
                CompoundKind::Tuple => writeln!(dst, "(")?,
                CompoundKind::Map => writeln!(dst, "{{")?,
                CompoundKind::NominalTuple(name) => writeln!(dst, "{}(", name)?,
                CompoundKind::NominalStruct(name) => writeln!(dst, "{} {{", name)?,
                CompoundKind::NominalUnit(name) => {
                    match entries_count {
                        0 => write!(dst, "{}", name)?,
                        _ => panic!("'nominal unit' accepts no entry"),
                    }
                    break 'comma_joined;
                }
                CompoundKind::Maybe => {
                    match entries_count {
                        0 => panic!("'maybe value' must be some when expand has been triggered"),
                        1 => write!(dst, "? {}", entries.next().unwrap())?,
                        _ => panic!("'maybe value' accepts at most one entry"),
                    }
                    break 'comma_joined;
                }
                CompoundKind::MapPair => {
                    match entries_count {
                        0 => panic!("'map pair' must not missing a key when expand has been triggered"),
                        1 => write!(dst, "{} => ", entries.next().unwrap())?,
                        2 => write!(dst, "{} => {}", entries.next().unwrap(), entries.next().unwrap())?,
                        _ => panic!("'map pair' accepts exactly two entries"),
                    }
                    break 'comma_joined;
                }
                CompoundKind::StructField => {
                    match entries_count {
                        0 => panic!("'struct field' must not missing a name when expand has been triggered"),
                        1 => write!(dst, "{}: ", entries.next().unwrap())?,
                        2 => write!(dst, "{}: {}", entries.next().unwrap(), entries.next().unwrap())?,
                        _ => panic!("'struct field' accepts exactly two entries"),
                    }
                    break 'comma_joined;
                }
            }
            for entry in entries {
                write_indent(dst, depth + 1)?;
                writeln!(dst, "{},", entry)?;
            }
        }

        let delta_width = comp.cumulative_width;
        for nested_comp in &mut self.stack[depth..] {
            nested_comp.cumulative_width -= delta_width;
        }

        Ok(())
    }

    #[inline]
    fn direct_write(&mut self, entry: String) -> fmt::Result {
        debug_assert!(!self.stack.is_empty());

        let dst = &mut self.dst;
        let write_indent =
            |dst: &mut W| (0..self.cfg.indent_width * self.stack.len()).try_for_each(|_| dst.write_str(" "));

        match self.stack.last().unwrap().kind {
            CompoundKind::Tuple
            | CompoundKind::Seq
            | CompoundKind::Map
            | CompoundKind::NominalTuple(_)
            | CompoundKind::NominalStruct(_) => {
                write_indent(dst)?;
                writeln!(dst, "{},", entry)?;
            }
            CompoundKind::Maybe | CompoundKind::NominalUnit(_) => unreachable!(),
            CompoundKind::MapPair | CompoundKind::StructField => (),
        }

        Ok(())
    }
}

#[doc(hidden)]
pub struct SerriaVisitor<'w, W: Write> {
    ser: &'w mut Serria<W>,
}

impl<W: Write> Drop for SerriaVisitor<'_, W> {
    fn drop(&mut self) {
        let mut deferred_err = false;

        if let Some(comp) = self.stack.pop() {
            debug_assert!(comp.ctrl == LayoutControl::Compact);

            let entries_count = self.queue.len() - comp.queue_index;
            let mut entries = self.queue.drain(comp.queue_index..);
            let mut stringified = String::new();

            deferred_err |= || -> fmt::Result {
                match &comp.kind {
                    CompoundKind::Seq => write!(stringified, "[")?,
                    CompoundKind::Tuple => write!(stringified, "(")?,
                    CompoundKind::Map => {
                        write!(stringified, "{{")?;
                        if entries_count > 0 {
                            write!(stringified, " ")?;
                        }
                    }

                    CompoundKind::NominalTuple(name) => write!(stringified, "{}(", name)?,
                    CompoundKind::NominalStruct(name) => {
                        write!(stringified, "{} {{", name)?;
                        if entries_count > 0 {
                            write!(stringified, " ")?;
                        }
                    }
                    CompoundKind::NominalUnit(name) => {
                        match entries_count {
                            0 => write!(stringified, "{}", name)?,
                            _ => panic!("'nominal unit' accepts no entry"),
                        }
                        return Ok(());
                    }

                    CompoundKind::Maybe => {
                        match entries_count {
                            0 => write!(stringified, "?")?,
                            1 => write!(stringified, "? {}", entries.next().unwrap())?,
                            _ => panic!("'maybe value' accepts at most one entry"),
                        }
                        return Ok(());
                    }

                    CompoundKind::MapPair => {
                        match entries_count {
                            0 => panic!("'map pair' must not missing a key"),
                            1 => panic!("'map pair' must not missing a value"),
                            2 => write!(
                                stringified,
                                "{} => {}",
                                entries.next().unwrap(),
                                entries.next().unwrap()
                            )?,
                            _ => panic!("'map pair' accepts exactly two entries"),
                        }
                        return Ok(());
                    }

                    CompoundKind::StructField => {
                        match entries_count {
                            0 => panic!("'struct field' must not missing a name"),
                            1 => panic!("'struct field' must not missing a value"),
                            2 => write!(
                                stringified,
                                "{}: {}", //
                                entries.next().unwrap(),
                                entries.next().unwrap()
                            )?,
                            _ => panic!("'struct field' accepts exactly two entries"),
                        }
                        return Ok(());
                    }
                }

                for _ in 0..entries_count.saturating_sub(1) {
                    write!(stringified, "{}, ", entries.next().unwrap())?;
                }
                if let Some(entry) = entries.next() {
                    write!(stringified, "{}", entry)?;
                }

                match &comp.kind {
                    CompoundKind::Seq => write!(stringified, "]")?,
                    CompoundKind::Tuple | CompoundKind::NominalTuple(_) => write!(stringified, ")")?,
                    CompoundKind::Map | CompoundKind::NominalStruct(_) => {
                        if entries_count > 0 {
                            write!(stringified, " ")?;
                        }
                        write!(stringified, "}}")?;
                    }
                    _ => unreachable!(),
                }

                drop(entries);
                Ok(())
            }()
            .is_err();

            deferred_err |= self.push(stringified).is_err();
        } else {
            // match entries_count {
            //     1 => {
            //         let entry = self.queue.pop_back().unwrap();
            //         self.dst.write_str(&entry).err().and(Some(Reason::Write))
            //     }
            //     0 => Some(Reason::TooFewEntries),
            //     _ => Some(Reason::TooManyEntries),
            // }
            todo!()
        }

        self.deferred_err |= deferred_err;
    }
}

impl<W: Write> Deref for SerriaVisitor<'_, W> {
    type Target = Serria<W>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.ser
    }
}

impl<W: Write> DerefMut for SerriaVisitor<'_, W> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ser
    }
}

//------------------------------------------------------------------------------
