use alloc::{boxed::Box, string::String};
use core::fmt;

pub type Result<T = ()> = core::result::Result<T, Error>;

pub(crate) type ResultKind<T = ()> = core::result::Result<T, ErrorImpl>;

//==================================================================================================

#[derive(Debug)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub position: Position,
}

impl From<ErrorImpl> for Error {
    fn from(kind: ErrorImpl) -> Self {
        Self {
            kind: *kind.0,
            position: Position { line: 0, column: 0 },
        }
    }
}

#[derive(Debug)]
pub(crate) struct ErrorImpl(pub(crate) Box<ErrorKind>);

#[non_exhaustive]
#[derive(Debug)]
pub enum ErrorKind {
    Corrupted,
    ExceededRecursionLimit,
    UnexpectedEof,
    UnexpectedCarriageReturn,
    UnbalancedRawTicks,

    ExpectedDifferentEnumName { expected: &'static str, found: String },
    ExpectedDifferentStructName { expected: &'static str, found: String },

    ExpectedString,
    ExpectedCharacter,
    ExpectedByteString,
    ExpectedInt8,
    ExpectedInt16,
    ExpectedInt32,
    ExpectedInt64,
    ExpectedInt128,
    ExpectedUInt8,
    ExpectedUInt16,
    ExpectedUInt32,
    ExpectedUInt64,
    ExpectedUInt128,
    ExpectedFloat32,
    ExpectedFloat64,
    ExpectedBoolean,
    InvalidNumber(&'static str),
    InvalidNumberSuffix,
    InvalidNumberSpecial,
    InvalidParagraphLineInitiator,
    IntegerOverflow,
    IntegerUnderflow,

    ExpectedUnit,
    ExpectedUnitEnd,
    ExpectedMaybe,
    ExpectedArray,
    ExpectedArrayEnd,
    ExpectedTuple,
    ExpectedTupleEnd,
    ExpectedMapLike,
    ExpectedMapLikeEnd,
    UnexpectedUnitBody,

    ExpectedScalar,
    ExpectedRangeDotDot,
    ExpectedRangeDotDotEq,
    UnexpectedRangeDotDotEq,

    ExpectedQuote,
    ExpectedUnquote,
    ExpectedColon,
    ExpectedFatArrow,
    ExpectedDelimiter,
    InvalidNominalBody,

    DuplicatedComma,

    UnclosedBlockComment,
    InvalidByteEscape,
    InvalidAsciiEscape,
    InvalidUnicodeEscape,
    InvalidUtf8Sequence,
    UnexpectedNonAsciiCharacter,
    UnexpectedControlCharacter,
    ExpectedIdentifier,
    UnexpectedKeywordAsIdentifier,
    UnexpectedUnderscoreIdentifier,

    ExpectedEndOfInput,
    ExpectedSemicolonOrEndOfInput,
}

impl serde::de::StdError for ErrorImpl {}

impl serde::de::Error for ErrorImpl {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        todo!()
    }
}

impl fmt::Display for ErrorImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl From<ErrorKind> for ErrorImpl {
    fn from(kind: ErrorKind) -> Self {
        Self(Box::new(kind))
    }
}

impl From<lexical_util::Error> for ErrorImpl {
    fn from(value: lexical_util::Error) -> Self {
        todo!()
    }
}

impl From<simdutf8::basic::Utf8Error> for ErrorImpl {
    fn from(value: simdutf8::basic::Utf8Error) -> Self {
        todo!()
    }
}

impl From<simdutf8::compat::Utf8Error> for ErrorImpl {
    fn from(value: simdutf8::compat::Utf8Error) -> Self {
        todo!()
    }
}

impl From<data_encoding::DecodeKind> for ErrorImpl {
    fn from(value: data_encoding::DecodeKind) -> Self {
        todo!()
    }
}
