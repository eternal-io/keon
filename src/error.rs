use chumsky::prelude::*;
use core::{fmt, ops::Range};

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub pos: usize,
    pub kind: ErrorKind,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    WontImplement,

    UnderscoreIdent,
    DeeplyNestedComment,
    ThickRawEnclosure,
    NonAsciiByteString,
    MultilineNormalString,
    BrokenParagraph,

    InvalidEscape,
    InvalidByteEscape,
    InvalidAsciiEscape,
    InvalidUnicodeEscape,
    InvalidBytesEncoding(data_encoding::DecodeKind),
    InvalidNumberFound(lexical_core::Error),

    ExpectedEnd,
    UnexpectedEnd,
    ExpectedSemiOrEnd,
    ExpectedIdent,
    UnexpectedKeyword { keyword: &'static str },
    ExpectedOption,
    ExpectedBoolean,
    ExpectedCharacter,
    ExpectedString,
    ExpectedByteInteger,
    ExpectedByteString,
    ExpectedSequence,
    ExpectedTuple,
    ExpectedMap,
    ExpectedUnit,
    ExpectedUnitStruct { name: &'static str },
    ExpectedNewtypeStruct { name: &'static str },
    ExpectedTupleStruct { name: &'static str },
    ExpectedStruct { name: &'static str },
    ExpectedEnum { name: &'static str },
    ExpectedVariant { variants: &'static [&'static str] },
    ExpectedUnitVariant,
    ExpectedNewtypeVariant,
    ExpectedTupleVariant,
    ExpectedStructVariant,
    Expected(&'static str),
}

//------------------------------------------------------------------------------

impl Error {
    pub(crate) const fn new_at(pos: usize, kind: ErrorKind) -> Self {
        Self { pos, kind }
    }

    pub(crate) const fn raise_at<T>(pos: usize, kind: ErrorKind) -> Result<T> {
        Err(Self::new_at(pos, kind))
    }

    // pub(crate) const fn with_kind(mut self, kind: ErrorKind) -> Self {
    //     self.kind = kind;
    //     self
    // }
}

// impl TryFrom<Vec<Error>> for Error {
//     type Error = ();

//     fn try_from(value: Vec<Error>) -> ::core::result::Result<Self, Self::Error> {
//         value.into_iter().next().ok_or(())
//     }
// }

// impl<'a, I: Input<'a>> chumsky::error::Error<'a, I> for Error {}

// impl<'a, I: Input<'a>, L> chumsky::error::LabelError<'a, I, L> for Error {
//     fn expected_found<E: IntoIterator<Item = L>>(
//         expected: E,
//         found: Option<chumsky::util::MaybeRef<'a, I::Token>>,
//         span: I::Span,
//     ) -> Self {
//         todo!()
//     }
// }

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

impl From<data_encoding::DecodeError> for Error {
    fn from(e: data_encoding::DecodeError) -> Self {
        Self {
            pos: e.position,
            kind: e.kind.into(),
        }
    }
}

//------------------------------------------------------------------------------

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl From<lexical_core::Error> for ErrorKind {
    fn from(e: lexical_core::Error) -> Self {
        ErrorKind::InvalidNumberFound(e)
    }
}

impl From<data_encoding::DecodeKind> for ErrorKind {
    fn from(e: data_encoding::DecodeKind) -> Self {
        ErrorKind::InvalidBytesEncoding(e)
    }
}
