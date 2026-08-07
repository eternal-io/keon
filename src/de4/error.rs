use alloc::string::String;
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

#[non_exhaustive]
#[derive(Debug)]
pub enum ErrorKind {
    Corrupted,
    WontImplement,
    ExceededRecursionLimit,
    UnexpectedEof,
    UnexpectedCarriageReturn,
    UnbalancedRawTicks,

    ExpectedVariantName,
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
    InvalidNumberSuffix,
    InvalidNumberSpecial,
    InvalidParagraphLineInitiator,

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
}

impl serde::de::StdError for ErrorKind {}

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

impl From<lexical_core::Error> for ErrorKind {
    fn from(value: lexical_core::Error) -> Self {
        todo!()
    }
}

impl From<simdutf8::compat::Utf8Error> for ErrorKind {
    fn from(value: simdutf8::compat::Utf8Error) -> Self {
        todo!()
    }
}

impl From<data_encoding::DecodeKind> for ErrorKind {
    fn from(value: data_encoding::DecodeKind) -> Self {
        todo!()
    }
}
