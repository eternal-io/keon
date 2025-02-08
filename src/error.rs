use core::{fmt, num::NonZeroU16};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub to: Option<(NonZeroU16, NonZeroU16)>,
    pub kind: ErrorKind,
    pub from: Option<(NonZeroU16, NonZeroU16)>,
    pub want: Option<OriginallyWant>,
}

impl Error {
    pub(crate) const fn new(kind: ErrorKind) -> Self {
        Self {
            to: None,
            kind,
            from: None,
            want: None,
        }
    }

    pub(crate) const fn new_detailed(msg: String) -> Self {
        Self::new(ErrorKind::Detailed(msg))
    }

    pub(crate) const fn raise<T>(kind: ErrorKind) -> Result<T> {
        Err(Self::new(kind))
    }

    pub(crate) const fn raise_working<T>(kind: ErrorKind, original: OriginallyWant) -> Result<T> {
        Err(Self::new(kind).want(original))
    }

    pub(crate) const fn want(mut self, original: OriginallyWant) -> Self {
        self.want = Some(original);
        self
    }
}

impl kaparser::Situate for Error {
    fn situate(&mut self, to: (NonZeroU16, NonZeroU16), from: Option<(NonZeroU16, NonZeroU16)>) {
        self.to = Some(to);
        self.from = from;
    }
}

impl core::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // let Error { line, col, kind } = self;

        // if let Some(n) = line {
        //     write!(f, ":{}", n)?;
        //     if let Some(m) = col {
        //         write!(f, ":{} ", m)?
        //     }
        // }

        // write!(f, "{}", kind)

        todo!()
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::new_detailed(format!("(serialize) {:#}", msg))
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::new_detailed(format!("(deserialize) {:#}", msg))
    }
}

impl From<kaparser::Utf8Error> for Error {
    fn from(e: kaparser::Utf8Error) -> Self {
        Self::new_detailed(format!("(decode) {:#}", e))
    }
}

impl From<lexical_core::Error> for Error {
    fn from(e: lexical_core::Error) -> Self {
        Self::new(ErrorKind::InvalidNumber(e))
    }
}

impl From<data_encoding::DecodeError> for Error {
    fn from(e: data_encoding::DecodeError) -> Self {
        Self::new(ErrorKind::InvalidBytesEncoding(e))
    }
}

impl From<Box<dyn core::error::Error>> for Error {
    fn from(e: Box<dyn core::error::Error>) -> Self {
        let e = match e.downcast::<kaparser::Utf8Error>() {
            Ok(e) => return Self::from(*e),
            Err(e) => e,
        };

        #[cfg(feature = "std")]
        let e = match e.downcast::<std::io::Error>() {
            Ok(e) => return Self::new_detailed(format!("(IO) {:#}", e)),
            Err(e) => e,
        };

        Self::new_detailed(format!("(other) {:#}", e))
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedEof,
    UnexpectedToken,
    UnexpectedNewline,
    UnexpectedNonAscii,
    UnexpectedUnicodeEscape,
    UnbalancedLiteralClose,
    InvalidNumber(lexical_core::Error),
    InvalidCharacterTooLess,
    InvalidCharacterTooMany,
    InvalidStringEscape,
    InvalidBytesEscape,
    InvalidBytesEncoding(data_encoding::DecodeError),
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

    ExceededRecursionLimit,

    Detailed(String),
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
            InvalidAsciiEscape => write!(f, "ASCII hex escape code must be at most 0x7F"),
            InvalidUnicodeEscape => write!(f, "Unicode escape code muse be hexadecimal and at most 10FFFF"),
            ExpectedComma => write!(f, "expected comma"),
            ExpectedFatArrow => write!(f, "expected fat arrow"),
            ExpectedNonUnitStruct => write!(f, "expected non-unit struct (newtype, tuple or map)"),
            ExpectedVariant => write!(f, "expected variant (an identifier)"),
            ExpectedUnitVariant => write!(f, "expected unit variant"),
            ExpectedNewtypeVariant => write!(f, "expected newtype variant"),
            ExpectedTupleVariant => write!(f, "expected tuple variant"),
            ExpectedStructVariant => write!(f, "expected struct variant"),
            ExpectedEof => write!(f, "expected EOF"),

            ExceededRecursionLimit => write!(f, "exceeded recursion limit"),

            Detailed(msg) => f.write_str(msg),

            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginallyWant {
    Identifier,

    LiteralCharacter,

    LiteralSignedInteger,
    LiteralUnsignedInteger,
    LiteralFloatNumber,

    LiteralString,
    LiteralStringRaw,

    LiteralBytes,
    LiteralBytesRaw,
    LiteralBytesEncoding,
}

impl fmt::Display for OriginallyWant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use OriginallyWant::*;
        match self {
            LiteralCharacter => write!(f, "character literal"),
            _ => todo!(),
        }
    }
}
