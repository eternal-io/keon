use self::private::*;
use super::*;

pub enum Indicator<'de> {
    Unit,
    Bool(bool),
    Char(char),
    Bytes(BytesKind),
    Number(NumberKind),
    String(StringKind),
    Maybe,
    PunctStart(PunctStart),
    NominalPath(NominalPathRef<'de>),
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PunctStart {
    /** `(` */ Paren,
    /** `[` */ Brack,
    /** `{` */ Brace,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PunctDelim {
    /** `)` */ Paren,
    /** `]` */ Brack,
    /** `}` */ Brace,
    /** `,` */ Comma,
    /** `:` */ Colon,
    /** `=>`*/ FatArrow,
    /** `;` */ Semicolon,
               EOF,
}

impl PunctStart {
    pub(crate) fn expect(&self, punct: Self) -> ResultKind {
        (*self == punct).then_some(()).ok_or_else(|| match punct {
            PunctStart::Paren => todo!(),
            PunctStart::Brack => todo!(),
            PunctStart::Brace => todo!(),
        })
    }
}

impl PunctDelim {
    pub(crate) fn expect(&self, punct: Self) -> ResultKind {
        (*self == punct).then_some(()).ok_or_else(|| match punct {
            PunctDelim::Paren => todo!(),
            PunctDelim::Brack => todo!(),
            PunctDelim::Brace => todo!(),
            PunctDelim::Comma => todo!(),
            PunctDelim::Colon => todo!(),
            PunctDelim::FatArrow => todo!(),
            PunctDelim::Semicolon => todo!(),
            PunctDelim::EOF => todo!(),
        })
    }

    pub(crate) fn expects(&self, puncts: &[Self], reason: ErrorKind) -> ResultKind {
        puncts.contains(self).then_some(()).ok_or(reason)
    }
}

pub enum NumberKind {
    Byte,
    Digit,
    Negative,
    Infinity,
    NotANumber,
}

pub enum StringKind {
    Normal,
    Raw(usize),
    Paragraph(usize),
}

pub enum BytesKind {
    Normal,
    Raw(usize),
    Base64,
    Base32,
    Base16,
}

mod private {
    pub trait Sealed {}
}

macro_rules! parse_concr {
    ( $method:ident, $ty:ty ) => {
        #[doc(hidden)]
        fn $method(&mut self) -> ResultKind<$ty>;
    };
}

/*
    NOTE:
    Implementor methods that start with `begin_` or `seek_`, must `eat_ws` first,
    and then `set_position` before consume leading content.
*/
pub trait Source<'de>: Sealed {
    fn set_position(&mut self);

    fn get_position(&self) -> Position;

    fn begin(&mut self) -> ResultKind<Indicator<'de>>;

    fn begin_unit(&mut self) -> ResultKind;

    fn begin_bool(&mut self) -> ResultKind<bool>;

    fn begin_char(&mut self) -> ResultKind;

    /// Only [`NumberKind::Byte`] consume leading content here.
    fn begin_number(&mut self) -> ResultKind<NumberKind>;

    fn begin_string(&mut self) -> ResultKind<StringKind>;

    fn begin_bytes(&mut self) -> ResultKind<BytesKind>;

    fn begin_maybe(&mut self) -> ResultKind;

    fn begin_sequence(&mut self) -> ResultKind;

    fn begin_tuple(&mut self) -> ResultKind;

    fn begin_map(&mut self) -> ResultKind;

    fn begin_nominal<'t>(&mut self, buf: &'t mut Vec<u8>) -> ResultKind<NominalPathRef<'t>>
    where
        'de: 't;

    fn begin_identifier<'t>(&mut self, buf: &'t mut Vec<u8>) -> ResultKind<&'t str>
    where
        'de: 't;

    /// Consume the leading whitespace and comments.
    fn eat_ws(&mut self) -> ResultKind;

    /// Consume the seeked 'delim'. Only takes effect after `seek_delim`.
    fn eat_delim(&mut self);

    /// Seek the next 'delim' without consume it. Returns `None` if not found.
    fn seek_delim(&mut self) -> ResultKind<Option<PunctDelim>>;

    /// Seek the next 'delim' without consume it. Returns `Err` if not found.
    fn seek_delim_expected(&mut self) -> ResultKind<PunctDelim> {
        self.seek_delim()?.ok_or(ErrorKind::ExpectedDelimiter)
    }

    parse_concr!(parse_char, char);
    parse_concr!(parse_byte, u8);
    parse_concr!(parse_i8, i8);
    parse_concr!(parse_i16, i16);
    parse_concr!(parse_i32, i32);
    parse_concr!(parse_i64, i64);
    parse_concr!(parse_i128, i128);
    parse_concr!(parse_u8, u8);
    parse_concr!(parse_u16, u16);
    parse_concr!(parse_u32, u32);
    parse_concr!(parse_u64, u64);
    parse_concr!(parse_u128, u128);
    parse_concr!(parse_f32, f32);
    parse_concr!(parse_f64, f64);

    fn parse_number(&mut self) -> ResultKind<Either<Number2, NumberNoSuffix2>>;

    fn parse_string<'t>(&mut self, kind: StringKind, buf: &'t mut Vec<u8>) -> ResultKind<Either<&'de str, &'t str>>;

    fn parse_bytes<'t>(&mut self, kind: BytesKind, buf: &'t mut Vec<u8>) -> ResultKind<Either<&'de [u8], &'t [u8]>>;
}

//==================================================================================================
