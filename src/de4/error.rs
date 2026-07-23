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

    // ExpectedIdentifier,
    // ExpectedNominalPath,
    ExpectedVariantName,
    ExpectedDifferentEnumName { expected: &'static str, found: String },
    ExpectedDifferentStructName { expected: &'static str, found: String },
    UnexpectedPathAsStructName,

    ExpectedMaybe,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedTuple,
    ExpectedTupleEnd,
    ExpectedMapLike,
    ExpectedMapLikeEnd,

    ExpectedColon,
    ExpectedFatArrow,
    ExpectedInitiator,

    DuplicatedComma,

    InvalidUnit,
    InvalidNominalStructureBody,
    InvalidUtf8Character,
    UnclosedBlockComment,
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
