use std::{fmt, io, num::NonZeroU32};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct Error {
    pub line: Option<NonZeroU32>,
    pub col: Option<NonZeroU32>,
    pub kind: ErrorKind,
}
impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            line: None,
            col: None,
            kind,
        }
    }
    pub(crate) fn raise<T>(kind: ErrorKind) -> Result<T> {
        Err(Self::new(kind))
    }
}
impl std::error::Error for Error {}
impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::new(ErrorKind::Serialize(msg.to_string()))
    }
}
impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::new(ErrorKind::Deserialize(msg.to_string()))
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Error { line, col, kind } = self;
        if let Some(n) = line {
            write!(f, ":{}", n)?;
            match col {
                Some(m) => write!(f, ":{} ", m)?,
                None => write!(f, ":-1 ")?,
            }
        }
        write!(f, "{}", kind)
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::new(ErrorKind::Io(e.to_string()))
    }
}
impl From<kaparser::Utf8Error> for Error {
    fn from(e: kaparser::Utf8Error) -> Self {
        Error::new(ErrorKind::Utf8(e.position()))
    }
}
impl From<Box<dyn std::error::Error>> for Error {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        let e = match e.downcast::<kaparser::Utf8Error>() {
            Ok(e) => return Self::from(*e),
            Err(e) => e,
        };

        let e = match e.downcast::<std::io::Error>() {
            Ok(e) => return Self::from(*e),
            Err(e) => e,
        };

        Self::new(ErrorKind::Custom(e))
    }
}

#[non_exhaustive]
#[derive(Debug, Default)]
pub enum ErrorKind {
    UnexpectedEof,
    #[default]
    UnexpectedToken,
    UnexpectedNewline,
    UnexpectedNonAscii,
    UnexpectedUnicodeEscape,
    UnbalancedLiteralClose,
    InvalidNumber(lexical_core::Error),
    InvalidCharacterTooLess,
    InvalidCharacterTooMany,
    InvalidBytesEncoding(data_encoding::DecodeError),
    InvalidEscape,
    InvalidAsciiEscape,
    InvalidUnicodeEscape,

    ExpectedComma,
    ExpectedFatArrow,
    ExpectedNonUnitStruct,
    ExpectedVariant,
    ExpectedUnitVariant,
    ExpectedNewtypeVariant,
    ExpectedTupleVariant,
    ExpectedStructVariant,
    ExpectedEof,

    Io(String),
    Utf8(usize),
    Custom(Box<dyn std::error::Error>),
    Serialize(String),
    Deserialize(String),

    ExceededRecursionLimit,
}
impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ErrorKind::*;
        match self {
            UnexpectedEof => write!(f, "unexpected EOF"),
            UnexpectedToken => write!(f, "unexpected token"),
            UnexpectedNewline => write!(f, "this literal does not allow break, use `\\n` instead"),
            UnexpectedNonAscii => write!(f, "unexpected non ascii in byte string"),
            UnexpectedUnicodeEscape => write!(f, "unexpected unicode escape in byte string"),
            UnbalancedLiteralClose => write!(f, "unbalanced literal close"),
            InvalidNumber(e) => write!(f, "{}", e),
            InvalidCharacterTooLess => write!(f, "character literal must contain one codepoint"),
            InvalidCharacterTooMany => write!(f, "character literal may only contain one codepoint"),
            InvalidBytesEncoding(e) => write!(f, "{}", e),
            InvalidEscape => write!(f, "invalid escape"),
            InvalidAsciiEscape => write!(f, "ASCII hex escape code must be at most 0x7F"),
            InvalidUnicodeEscape => write!(f, "Unicode escape code muse be at most 10FFFF"),

            ExpectedComma => write!(f, "expected comma"),
            ExpectedFatArrow => write!(f, "expected fat arrow"),
            ExpectedNonUnitStruct => write!(f, "expected non-unit struct (newtype, tuple or map)"),
            ExpectedVariant => write!(f, "expected variant (an identifier)"),
            ExpectedUnitVariant => write!(f, "expected unit variant"),
            ExpectedNewtypeVariant => write!(f, "expected newtype variant"),
            ExpectedTupleVariant => write!(f, "expected tuple variant"),
            ExpectedStructVariant => write!(f, "expected struct variant"),
            ExpectedEof => write!(f, "expected EOF"),

            Io(e) => write!(f, "(IO) {}", e),
            Utf8(n) => todo!(),
            Custom(e) => write!(f, "(custom) {}", e),
            Serialize(e) => write!(f, "(serialize) {}", e),
            Deserialize(e) => write!(f, "(deserialize) {}", e),

            ExceededRecursionLimit => write!(f, "exceeded recursion limit"),
        }
    }
}
