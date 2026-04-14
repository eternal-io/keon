use super::*;

#[cfg(feature = "std")]
use std::io;

pub trait Source<'de> {
    fn next(&mut self) -> Result<Option<u8>>;

    fn position(&self) -> Position;
}

pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[cfg(feature = "std")]
pub struct IoRead<R: io::Read> {
    src: R,
}

pub struct Bytes<'de> {
    src: &'de [u8],
}

pub struct Str<'de> {
    src: &'de str,
}
