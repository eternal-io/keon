use self::{error::*, source::*};
use crate::{value::*, PrivateMethod};
use alloc::{boxed::Box, vec::Vec};
use core::ops::{Deref, DerefMut};
use either::Either;

mod de_to_concr;
mod de_to_value;
pub mod error;
pub mod source;

pub fn parse_str<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T> {
    Deserializer::new(SliceRead::from_str(s)).deserialize()
}
pub fn parse_bytes<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T> {
    Deserializer::new(SliceRead::from_bytes(bytes)).deserialize()
}

pub fn from_str(s: &str) -> Result<Value> {
    Deserializer::new(SliceRead::from_str(s)).deserialize()
}
pub fn from_bytes(bytes: &[u8]) -> Result<Value> {
    Deserializer::new(SliceRead::from_bytes(bytes)).deserialize()
}

//==================================================================================================

#[expect(private_interfaces)]
pub trait Deserialize<'de>: Sized {
    #[doc(hidden)]
    fn deserialize_with<R: Read<'de>>(der: &mut Deserializer<R>, _: PrivateMethod) -> ResultKind<Self>;
}

pub struct Deserializer<R> {
    src: R,
    ttl: usize,
    buf: Vec<u8>,
}

impl<R> Deref for Deserializer<R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        &self.src
    }
}

impl<R> DerefMut for Deserializer<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.src
    }
}

impl<R> Deserializer<R> {
    pub fn new(src: R) -> Self {
        Self {
            src,
            ttl: 160,
            buf: Vec::new(),
        }
    }

    pub fn corrupted(&self) -> bool {
        self.ttl == 0
    }

    fn enter_nesting(&mut self) -> ResultKind {
        if self.ttl > 0 {
            self.ttl -= 1;
            Ok(())
        } else {
            raise(ErrorKind::ExceededRecursionLimit)
        }
    }

    fn exit_nesting(&mut self) {
        self.ttl += 1;
    }
}

impl<'de, R: Read<'de>> Deserializer<R> {
    pub fn deserialize<T: Deserialize<'de>>(&mut self) -> Result<T> {
        let val = self.deserialize_partial::<T>()?;
        self.finish_all().map_err(self.fixing_pos())?;
        Ok(val)
    }

    pub fn deserialize_one<T: Deserialize<'de>>(&mut self) -> Result<T> {
        let val = self.deserialize_partial::<T>()?;
        self.finish_one().map_err(self.fixing_pos())?;
        Ok(val)
    }

    pub fn finish(&mut self) -> Result {
        if self.ttl == 0 {
            return Err(Error {
                kind: ErrorKind::Corrupted,
                position: self.position(),
            });
        }
        self.finish_all().map_err(self.fixing_pos())
    }

    fn deserialize_partial<T: Deserialize<'de>>(&mut self) -> Result<T> {
        if self.ttl == 0 {
            return Err(Error {
                kind: ErrorKind::Corrupted,
                position: self.position(),
            });
        }
        T::deserialize_with(self, PrivateMethod).map_err(self.fixing_pos())
    }

    fn deserialize_scalar(&mut self) -> ResultKind<Scalar> {
        let scalar = match self.src.begin(&mut self.buf)? {
            Indicator::Char(ch) => Scalar::Char(ch),
            Indicator::Byte(byte) => Scalar::Number(byte.into()),
            Indicator::Number(kind) => Scalar::Number(self.parse_number(kind)?),
            _ => return raise(ErrorKind::ExpectedScalar),
        };
        Ok(scalar)
    }

    fn fixing_pos(&mut self) -> impl FnOnce(BoxedKind) -> Error + '_ {
        |e| {
            self.ttl = 0;
            Error {
                kind: *e.0,
                position: self.position(),
            }
        }
    }
}

fn raise<T>(kind: ErrorKind) -> ResultKind<T> {
    Err(BoxedKind(Box::new(kind)))
}
