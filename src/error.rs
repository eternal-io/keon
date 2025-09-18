use core::fmt;

pub type Result<T> = ::core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub pos: usize,
    pub kind: ErrorKind,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Corrupted,
    WontImplement,
    ExceededRecursionLimit,

    DeeplyNestedComment,
    NonAsciiByteString,
    UnbalancedRawDelimiters,
    UnexpectedCarriageReturn,

    InvalidEscape,
    InvalidByteEscape,
    InvalidAsciiEscape,
    InvalidUnicodeEscape,
    IntegerOverflow,
    IntegerUnderflow,
    InvalidNumberFound(lexical_core::Error),
    InvalidBytesEncoding(data_encoding::DecodeKind),
    InvalidParagraphLine,

    ExpectedEnd,
    UnexpectedEnd,
    ExpectedSemiOrEnd,
    ExpectedIdent,
    UnexpectedKeywordIdent { keyword: &'static str },
    UnexpectedUnderscoreIdent,
    ExpectedValue,
    ExpectedNominalValue,
    Expected(&'static str),

    /* parsing with known types */
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
