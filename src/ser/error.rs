use super::*;

pub type SeriaResult = ::core::result::Result<(), SeriaError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeriaError;

impl core::error::Error for SeriaError {}

impl serde::ser::Error for SeriaError {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        Self
    }
}

impl fmt::Display for SeriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
