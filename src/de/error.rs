use alloc::{boxed::Box, string::ToString};
use core::fmt;

pub type Result<T = ()> = core::result::Result<T, Error>;

//==================================================================================================

pub struct Error(Box<ErrorImpl>);

pub enum Category {
    Data,
    Syntax,
}

#[derive(Debug)]
pub(super) struct ErrorImpl {
    kind: ErrorKind,
    position: Position,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Position {
    pub line: usize,
    pub column: usize,
}

#[non_exhaustive]
#[derive(Debug)]
pub(super) enum ErrorKind {
    Message(Box<str>),
    Corrupted,
    ExceededRecursionLimit,
    UnclosedBlockComment,
    InvalidUtf8Sequence,
    ExpectedContent,
    ExpectedSemicolonOrEof,
    ExpectedEof,

    ExpectedColon,
    ExpectedFatArrow,
    ExpectedRangeDotDot,
    ExpectedRangeDotDotEq,
    UnexpectedRangeDotDotEq,
    ExpectedDelimiter,
    UnexpectedUnitBody,
    InvalidNominalBody,

    ExpectedScalar,
    ExpectedBoolean,
    ExpectedCharacter,
    ExpectedString,
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
    ExpectedUnit,
    ExpectedMaybe,
    ExpectedArray,
    ExpectedTuple,
    ExpectedMapLike,
    ExpectedIdentifier,
    UnexpectedKeywordAsIdentifier,
    UnexpectedUnderscoreIdentifier,

    DuplicatedComma,
    ExpectedUnitEnd,
    ExpectedArrayEnd,
    ExpectedTupleEnd,
    ExpectedMapLikeEnd,

    ExpectedQuote,
    ExpectedUnquote,
    UnbalancedRawTicks,
    InvalidParagraphLineInitiator,
    InvalidByteEscape,
    InvalidAsciiEscape,
    InvalidUnicodeEscape,
    UnexpectedControlCharacter,
    UnexpectedNonAsciiCharacter,
    UnexpectedCarriageReturn,

    IntegerOverflow,
    IntegerUnderflow,
    InvalidFloatSpecial,
    InvalidNumber(&'static str),
    InvalidNumberSuffix,

    InvalidDataEncodingLength,
    InvalidDataEncodingSymbol,
    InvalidDataEncodingTrailing,
    InvalidDataEncodingPadding,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ErrorKind::Message(ref msg) => msg,
            ErrorKind::Corrupted => "deserializer already corrupted",
            ErrorKind::ExceededRecursionLimit => "exceeded recursion limit",
            ErrorKind::UnclosedBlockComment => "unclosed block comment",
            ErrorKind::InvalidUtf8Sequence => "invalid UTF-8 sequence",
            ErrorKind::ExpectedContent => "expected content",
            ErrorKind::ExpectedSemicolonOrEof => "expected semicolon or end of input",
            ErrorKind::ExpectedEof => "expected end of input",

            ErrorKind::ExpectedColon => "expected colon",
            ErrorKind::ExpectedFatArrow => "expected `=>`",
            ErrorKind::ExpectedRangeDotDot => "expected `..`",
            ErrorKind::ExpectedRangeDotDotEq => "expected `..=`",
            ErrorKind::UnexpectedRangeDotDotEq => "unexpected `..=`",
            ErrorKind::ExpectedDelimiter => "expected delimiter",
            ErrorKind::UnexpectedUnitBody => "unexpected unit body",
            ErrorKind::InvalidNominalBody => "invalid nominal body",

            ErrorKind::ExpectedScalar => "expected scalar",
            ErrorKind::ExpectedBoolean => "expected boolean",
            ErrorKind::ExpectedCharacter => "expected character",
            ErrorKind::ExpectedString => "expected string",
            ErrorKind::ExpectedByteString => "expected byte string",
            ErrorKind::ExpectedInt8 => "expected int8",
            ErrorKind::ExpectedInt16 => "expected int16",
            ErrorKind::ExpectedInt32 => "expected int32",
            ErrorKind::ExpectedInt64 => "expected int64",
            ErrorKind::ExpectedInt128 => "expected int128",
            ErrorKind::ExpectedUInt8 => "expected uint8",
            ErrorKind::ExpectedUInt16 => "expected uint16",
            ErrorKind::ExpectedUInt32 => "expected uint32",
            ErrorKind::ExpectedUInt64 => "expected uint64",
            ErrorKind::ExpectedUInt128 => "expected uint128",
            ErrorKind::ExpectedFloat32 => "expected float32",
            ErrorKind::ExpectedFloat64 => "expected float64",
            ErrorKind::ExpectedUnit => "expected unit",
            ErrorKind::ExpectedMaybe => "expected maybe",
            ErrorKind::ExpectedArray => "expected array",
            ErrorKind::ExpectedTuple => "expected tuple",
            ErrorKind::ExpectedMapLike => "expected map-like structure",
            ErrorKind::ExpectedIdentifier => "expected identifier",
            ErrorKind::UnexpectedKeywordAsIdentifier => "unexpected keyword as identifier",
            ErrorKind::UnexpectedUnderscoreIdentifier => "unexpected underscore identifier",

            ErrorKind::DuplicatedComma => "duplicated comma",
            ErrorKind::ExpectedUnitEnd => "expected end of unit",
            ErrorKind::ExpectedArrayEnd => "expected end of array",
            ErrorKind::ExpectedTupleEnd => "expected end of tuple",
            ErrorKind::ExpectedMapLikeEnd => "expected end of map-like structure",

            ErrorKind::ExpectedQuote => "expected quote",
            ErrorKind::ExpectedUnquote => "expected unquote, not found till end",
            ErrorKind::UnbalancedRawTicks => "unbalanced raw ticks",
            ErrorKind::InvalidParagraphLineInitiator => "invalid paragraph line initiator",
            ErrorKind::InvalidByteEscape => "invalid byte escape",
            ErrorKind::InvalidAsciiEscape => "invalid ASCII escape",
            ErrorKind::InvalidUnicodeEscape => "invalid Unicode escape",
            ErrorKind::UnexpectedControlCharacter => "unexpected control character",
            ErrorKind::UnexpectedNonAsciiCharacter => "unexpected non-ASCII character",
            ErrorKind::UnexpectedCarriageReturn => "unexpected carriage return",

            ErrorKind::IntegerOverflow => "integer overflow",
            ErrorKind::IntegerUnderflow => "integer underflow",
            ErrorKind::InvalidFloatSpecial => "invalid float special",
            ErrorKind::InvalidNumber(desc) => return write!(f, "invalid number: {}", desc),
            ErrorKind::InvalidNumberSuffix => "invalid number suffix",

            ErrorKind::InvalidDataEncodingLength => "invalid data encoding length",
            ErrorKind::InvalidDataEncodingSymbol => "invalid data encoding symbol",
            ErrorKind::InvalidDataEncodingTrailing => "invalid data encoding trailing",
            ErrorKind::InvalidDataEncodingPadding => "invalid data encoding padding",
        })
    }
}

//==================================================================================================

impl Position {
    pub const NULL: Self = Position { line: 0, column: 0 };
}

impl Error {
    pub(super) fn with_position(mut self, position: Position) -> Self {
        self.0.position = position;
        self
    }

    pub fn classify(&self) -> Category {
        match self.0.kind {
            ErrorKind::Message(_) => Category::Data,
            _ => Category::Syntax,
        }
    }

    pub fn is_data(&self) -> bool {
        matches!(self.classify(), Category::Data)
    }

    pub fn is_syntax(&self) -> bool {
        matches!(self.classify(), Category::Syntax)
    }
}

impl serde::de::StdError for Error {}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self(Box::new(ErrorImpl {
            kind: ErrorKind::Message(msg.to_string().into_boxed_str()),
            position: Position::NULL,
        }))
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error")
            .field(&self.0.kind)
            .field(&format_args!(
                "Position {{ line: {}, column: {} }}",
                self.0.position.line, self.0.position.column
            ))
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ErrorImpl { kind, position } = &*self.0;
        if *position != Position::NULL {
            write!(f, ":{}:{} ", position.line, position.column)?;
        }
        write!(f, "{}", kind)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self(Box::new(ErrorImpl {
            kind,
            position: Position::NULL,
        }))
    }
}

impl From<lexical_util::Error> for Error {
    fn from(err: lexical_util::Error) -> Self {
        Self(Box::new(ErrorImpl {
            kind: match err {
                lexical_util::Error::Overflow(_) => ErrorKind::IntegerOverflow,
                lexical_util::Error::Underflow(_) => ErrorKind::IntegerUnderflow,
                lexical_util::Error::InvalidSpecial => ErrorKind::InvalidFloatSpecial,
                _ => ErrorKind::InvalidNumber(err.description()),
            },
            position: Position::NULL,
        }))
    }
}

impl From<data_encoding::DecodeKind> for Error {
    fn from(err: data_encoding::DecodeKind) -> Self {
        Self(Box::new(ErrorImpl {
            kind: match err {
                data_encoding::DecodeKind::Length => ErrorKind::InvalidDataEncodingLength,
                data_encoding::DecodeKind::Symbol => ErrorKind::InvalidDataEncodingSymbol,
                data_encoding::DecodeKind::Trailing => ErrorKind::InvalidDataEncodingTrailing,
                data_encoding::DecodeKind::Padding => ErrorKind::InvalidDataEncodingPadding,
            },
            position: Position::NULL,
        }))
    }
}
