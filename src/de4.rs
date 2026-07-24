use self::{error::*, source::*};
use crate::{format::*, value::*, PrivateMethod};
use either::Either;

mod de_to_concr;
mod de_to_value;
pub mod error;
pub mod source;

pub const DEFAULT_RECURSION_LIMIT: isize = 160;

pub trait Deserialize<'de>: Sized {
    #[expect(private_interfaces)]
    #[doc(hidden)]
    fn deserialize_with<R: Source<'de>>(der: &mut Deserializer<R>, _: PrivateMethod) -> ResultKind<Self>;
}

pub struct Deserializer<R> {
    src: R,
    ttl: isize, // A negative TTL means the deserializer is corrupted.
    buf: Vec<u8>,
}

impl<R> Deserializer<R> {
    pub fn new(src: R) -> Self {
        Self {
            src,
            ttl: DEFAULT_RECURSION_LIMIT,
            buf: Vec::new(),
        }
    }

    fn ttl_enter(&mut self) -> ResultKind {
        if self.ttl >= 0 {
            self.ttl -= 1;
            Ok(())
        } else {
            Err(ErrorKind::ExceededRecursionLimit)
        }
    }

    fn ttl_leave(&mut self) {
        self.ttl += 1;
    }
}

impl<'de, R: Source<'de>> Deserializer<R> {
    pub fn deserialize<T: Deserialize<'de>>(&mut self) -> Result<T> {
        if self.ttl < 0 {
            return Err(todo!("corrupted"));
        }

        let res = T::deserialize_with(self, PrivateMethod);

        // TODO: check no more contents?

        if res.is_err() {
            self.ttl = -999; // Mark the deserializer as corrupted.

            // TODO: fix error location.
        }

        todo!()
    }
}
