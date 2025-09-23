use core::fmt;

#[doc(alias = "ParseResult")]
pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[doc(alias = "ParseError")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub pos: usize,
    pub kind: ErrorKind,
}

#[non_exhaustive]
#[doc(alias = "ParseErrorKind")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /* special */
    Corrupted,
    WontImplement,
    DeeplyNestedComment,
    ExceededRecursionLimit,
    Deserialize(String),

    /* string related */
    UnbalancedRawDelimiters,
    UnexpectedCarriageReturn,
    UnexpectedNonAsciiCharacter,

    /* in detail */
    InvalidNumber,
    IntegerOverflow,
    IntegerUnderflow,
    InvalidNumberType,
    InvalidEscape,
    InvalidByteEscape,
    InvalidAsciiEscape,
    InvalidUnicodeEscape,
    InvalidParagraphLine,
    InvalidEncodingLength,
    InvalidEncodingTrailing,
    InvalidEncodingCharacter,

    /* structural */
    ExpectedEnd,
    UnexpectedEnd,
    ExpectedSemiOrEnd,
    ExpectedIdent,
    UnexpectedKeywordIdent(&'static str),
    UnexpectedUnderscoreIdent,
    ExpectedValue,
    ExpectedNominalValue,

    /* syntactic integrity */
    ExpectedQuote,
    ExpectedColon,
    ExpectedFatArrow,
    ExpectedBraceOpen,
    ExpectedBraceClose,
    ExpectedTupleClose,
    ExpectedSequenceClose,

    /* parsing with known types */
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
    ExpectedMaybe,
    ExpectedBoolean,
    ExpectedCharacter,
    ExpectedStringOrParagraph,
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
}

//------------------------------------------------------------------------------

impl Error {
    pub(crate) const fn new_at(pos: usize, kind: ErrorKind) -> Self {
        Self { pos, kind }
    }

    pub(crate) const fn raise_at<T>(pos: usize, kind: ErrorKind) -> Result<T> {
        Err(Self::new_at(pos, kind))
    }
}

impl core::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            pos: 0,
            kind: ErrorKind::Deserialize(msg.to_string()),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[doc(hidden)]
impl From<data_encoding::DecodeError> for Error {
    fn from(e: data_encoding::DecodeError) -> Self {
        Self {
            pos: e.position,
            kind: e.kind.try_into().expect("internal use should not panic"),
        }
    }
}

#[doc(hidden)]
impl From<lexical_core::Error> for Error {
    fn from(e: lexical_core::Error) -> Self {
        use lexical_core::Error::*;
        use ErrorKind::*;

        let kind = match e {
            Overflow(_) => IntegerOverflow,
            Underflow(_) => IntegerUnderflow,

            _ => InvalidNumber,
        };

        Self { pos: 0, kind }
    }
}

//------------------------------------------------------------------------------

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl TryFrom<data_encoding::DecodeKind> for ErrorKind {
    type Error = ();

    fn try_from(e: data_encoding::DecodeKind) -> Result<Self, Self::Error> {
        'ue: {
            use data_encoding::DecodeKind::*;
            use ErrorKind::*;

            let kind = match e {
                Length => InvalidEncodingLength,
                Symbol => InvalidEncodingCharacter,
                Trailing => InvalidEncodingTrailing,
                // Since padding is not currently allowed,
                // no padding related errors will be encountered.
                Padding => break 'ue,
            };

            return Ok(kind);
        }

        Err(())
    }
}
