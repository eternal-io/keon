use super::*;

pub type SeriaResult<T = ()> = ::core::result::Result<T, SeriaError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriaError {
    Write,
    Recursion,
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
