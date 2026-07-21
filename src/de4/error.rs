use core::fmt;

pub(crate) type Result<T = ()> = ::core::result::Result<T, Error>;

pub(crate) type ResultKind<T = ()> = ::core::result::Result<T, ErrorKind>;

//------------------------------------------------------------------------------

pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct Error {}

//==================================================================================================

#[derive(Debug)]
pub enum ErrorKind {
    Corrupted,
    WontImplement,
    ExceededRecursionLimit,

    ExpectedDelimiter,
    ExpectedIdentifier,
    ExpectedNominalPath,
    ExpectedVariantName,
    UnexpectedEnum {
        expected: &'static str,
        // found: String,
        // TODO!
    },
    ExpectedStruct {
        name: &'static str,
    },

    DuplicatedComma,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedColon,
    ExpectedFatArrow,
    ExpectedMapLike,
    ExpectedMapLikeEnd,
    ExpectedMaybe,
    ExpectedTuple,
    ExpectedTupleEnd,
    ExpectedUnit,
    ExpectedUnitEnd,
}

impl core::error::Error for ErrorKind {}

impl serde::de::Error for ErrorKind {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        todo!()
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
