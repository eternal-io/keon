use chumsky::prelude::*;
use core::fmt;

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    #[default]
    Foo,
}

//------------------------------------------------------------------------------

impl<'a, I: Input<'a>> chumsky::error::Error<'a, I> for Error {}

impl<'a, I: Input<'a>, L> chumsky::error::LabelError<'a, I, L> for Error {
    fn expected_found<E: IntoIterator<Item = L>>(
        expected: E,
        found: Option<chumsky::util::MaybeRef<'a, I::Token>>,
        span: I::Span,
    ) -> Self {
        todo!()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

//------------------------------------------------------------------------------

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
