use self::error::*;
use alloc::borrow::Cow;
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
    fn seria_to<W: Write>(&self, ser: &mut Serria<W>) -> SeriaResult;
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
    stack: Vec<StructuralTerm>,
    queue: Vec<String>,
    deferred_err: Option<SeriaError>,
}

enum LayoutControl {
    Horizontal,
    Vertical,
}

enum StructuralKind {
    Maybe,
    Tuple,
    Seq,
    Map,
    MapPair,
    NominalUnit,
    NominalTuple(String),
    NominalStruct(String),
    StructPair,
}

struct StructuralTerm {
    ctrl: LayoutControl,
    kind: StructuralKind,
    queue_index: usize,
    cumulative_width: usize,
}

impl<W: Write> Serria<W> {
    fn start(&mut self) -> SeriaResult<SerriaGuard<'_, W>> {
        self.flush_error()?;
        // TODO: write semicolon.
        Ok(SerriaGuard { serria: self })
    }

    fn flush_error(&mut self) -> SeriaResult {
        match self.deferred_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn write(&mut self, s: &str) -> SeriaResult {
        self.dst.write_str(s)?;
        Ok(())
    }

    fn write_indent(&mut self) -> SeriaResult {
        (0..self.now_indent_column()).try_for_each(|_| self.write(" "))?;
        Ok(())
    }

    fn now_indent_column(&self) -> usize {
        self.cfg.indent_width * self.stack.len()
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
        match self.stack.last_mut() {
            None => {
                todo!()
                // self.write(literal.as_ref())?;
                // self.write(";\n")?;
            }
            Some(StructuralTerm {
                kind,
                ctrl,
                queue_index,
                cumulative_width,
            }) => match kind {
                StructuralKind::Maybe => {
                    if queue_len > *queue_index {
                        return Err(SeriaError::TooManyEntries);
                    }
                    self.queue.push(literal);
                }
                StructuralKind::Tuple => todo!(),
                StructuralKind::Seq => todo!(),
                StructuralKind::Map => todo!(),
                StructuralKind::MapPair => todo!(),
                StructuralKind::NominalUnit => todo!(),
                StructuralKind::NominalTuple(_) => todo!(),
                StructuralKind::NominalStruct(_) => todo!(),
                StructuralKind::StructPair => todo!(),
            },
        }

        Ok(())
    }

    fn enter(&mut self, kind: StructuralKind) -> SeriaResult<SerriaGuard<'_, W>> {
        self.flush_error()?;

        todo!()
    }
}

impl<W: Write> Drop for SerriaGuard<'_, W> {
    fn drop(&mut self) {
        todo!()
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

pub struct SerriaConfig {
    indent_width: usize,
}
