use super::*;

pub type SeriaResult<T = ()> = ::core::result::Result<T, SeriaError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriaError {
    Write,
    Recursion,
    InvalidEntryKind,
    TooManyEntries,
}

impl core::error::Error for SeriaError {}

impl serde::ser::Error for SeriaError {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Self::Write
    }
}

impl fmt::Display for SeriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[doc(hidden)]
impl From<fmt::Error> for SeriaError {
    fn from(_: fmt::Error) -> Self {
        Self::Write
    }
}
