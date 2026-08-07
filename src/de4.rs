use self::{error::*, source::*};
use crate::{format::*, value::*, PrivateMethod};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};
use either::Either;

mod de_to_concr;
mod de_to_value;
pub mod error;
pub mod source;

pub trait Deserialize<'de>: Sized {
    #[expect(private_interfaces)]
    #[doc(hidden)]
    fn deserialize_with<R: Source<'de>>(der: &mut Deserializer<R>, _: PrivateMethod) -> ResultKind<Self>;
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
            Err(ErrorKind::ExceededRecursionLimit)
        }
    }

    fn exit_nesting(&mut self) {
        self.ttl += 1;
    }
}

impl<'de, R: Source<'de>> Deserializer<R> {
    pub fn deserialize<T: Deserialize<'de>>(&mut self) -> Result<T> {
        if self.ttl == 0 {
            return Err(todo!("corrupted"));
        }

        let res = T::deserialize_with(self, PrivateMethod);

        // TODO: check no more contents?

        if res.is_err() {
            self.ttl = 0; // Mark the deserializer as corrupted.

            // TODO: fix error location.
        }

        todo!()
    }
}
