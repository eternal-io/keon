use chumsky::prelude::*;
use core::{fmt, ops::Range};

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    WontImplement,

    UnclosedComment,
    DeeplyNestedComment,

    InvalidEscape,
    InvalidNumber(lexical_core::Error),

    ExpectedEnd,
    ExpectedSemiOrEnd,
    ExpectedByteInteger,
    ExpectedBoolean,
    ExpectedCharacter,
}

//------------------------------------------------------------------------------

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self { kind, span: 0..0 }
    }

    pub(crate) fn raise<T>(kind: ErrorKind) -> Result<T> {
        Err(Self::new(kind))
    }

    pub(crate) fn with_kind(mut self, kind: ErrorKind) -> Self {
        self.kind = kind;
        self
    }
}

impl TryFrom<Vec<Error>> for Error {
    type Error = ();

    fn try_from(value: Vec<Error>) -> ::core::result::Result<Self, Self::Error> {
        value.into_iter().next().ok_or(())
    }
}

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

impl core::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        todo!()
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        todo!()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl From<lexical_core::Error> for Error {
    fn from(e: lexical_core::Error) -> Self {
        Error::new(ErrorKind::InvalidNumber(e))
    }
}

//------------------------------------------------------------------------------

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
