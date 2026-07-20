use super::*;
use crate::Sealed;

pub(crate) enum Indicator<'de> {
    Unit,
    Bool(bool),
    Char(char),
    Number(NumberKind),
    String(StringKind),
    Bytes(BytesKind),
    Maybe,
    PunctStart(PunctStart),
    NominalPath(NominalPathRef<'de>),
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PunctStart {
    /** `(` */ Paren,
    /** `[` */ Brack,
    /** `{` */ Brace,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PunctDelim {
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

pub(crate) enum NumberKind {
    Byte,
    Digit,
    Negative,
    Infinity,
    NotANumber,
}

pub(crate) enum StringKind {
    Normal,
    Raw(usize),
    Paragraph(usize),
}

pub(crate) enum BytesKind {
    Normal,
    Raw(usize),
    Base64,
    Base32,
    Base16,
}

// Implementor methods that start with `begin_*` or `seek_*`, must `eat_ws` first,
// and then `set_position` before consuming the leading content.
//
// All `begin_*` methods shall consume leading contents.
#[expect(private_bounds, reason = "Sealed")]
pub trait Source<'de>: ReadConcr<'de> + ReadAny<'de> {}

pub(crate) trait ReadCommon<'de> {
    fn set_position(&mut self);

    /// Position of the most recent call to `set_position()`.
    fn position(&self) -> Position;

    /// Consumes the subsequent start punctuation. Returns `Err` if not found.
    fn start(&mut self, start: PunctStart) -> ResultKind;

    /// Consumes the subsequent delim punctuation. Returns `Err` if not found.
    fn delim(&mut self, delim: PunctDelim) -> ResultKind;

    /// Consumes the leading whitespace and comments.
    fn eat_ws(&mut self) -> ResultKind;

    /// Consumes the sought punctuation. Only takes effect after `seek_delim*`.
    fn eat_delim(&mut self);

    /// Seeks the next delim punctuation without consuming it. Returns `None` if not found.
    fn seek_delim(&mut self) -> ResultKind<Option<PunctDelim>>;

    /// Seeks the next delim punctuation without consuming it. Returns `Err` if not found.
    fn seek_delim_expected(&mut self) -> ResultKind<PunctDelim> {
        self.seek_delim()?.ok_or(ErrorKind::ExpectedDelimiter)
    }
}

pub(crate) trait ReadConcr<'de>: ReadCommon<'de> {
    fn parse_unit(&mut self) -> ResultKind {
        self.eat_ws()?;
        self.set_position();
        self.start(PunctStart::Paren)?;
        self.eat_ws()?;
        self.delim(PunctDelim::Paren)?;
        Ok(())
    }

    fn parse_bool(&mut self) -> ResultKind<bool>;

    fn begin_char(&mut self) -> ResultKind;

    /// If the subsequent content starts with `b'`, consume it and return true.
    fn begin_integer_try_byte(&mut self) -> bool;

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

    fn parse_string<'t>(&mut self, kind: StringKind, buf: &'t mut Vec<u8>) -> ResultKind<Either<&'de str, &'t str>>;

    fn parse_bytes<'t>(&mut self, kind: BytesKind, buf: &'t mut Vec<u8>) -> ResultKind<Either<&'de [u8], &'t [u8]>>;

    fn parse_byte(&mut self) -> ResultKind<u8>;

    fn parse_char(&mut self) -> ResultKind<char>;

    fn parse_i8(&mut self) -> ResultKind<i8>;
    fn parse_i16(&mut self) -> ResultKind<i16>;
    fn parse_i32(&mut self) -> ResultKind<i32>;
    fn parse_i64(&mut self) -> ResultKind<i64>;
    fn parse_i128(&mut self) -> ResultKind<i128>;

    fn parse_u8(&mut self) -> ResultKind<u8>;
    fn parse_u16(&mut self) -> ResultKind<u16>;
    fn parse_u32(&mut self) -> ResultKind<u32>;
    fn parse_u64(&mut self) -> ResultKind<u64>;
    fn parse_u128(&mut self) -> ResultKind<u128>;

    fn parse_f32(&mut self) -> ResultKind<f32>;
    fn parse_f64(&mut self) -> ResultKind<f64>;
}

pub(crate) trait ReadAny<'de>: ReadCommon<'de> {
    /// For [`NumberKind`], none of the variants consume the leading content (except for [`NumberKind::Byte`]),
    /// as the literal number will be passed to `lexical-core` for parsing.
    fn begin(&mut self) -> ResultKind<Indicator<'de>>;

    fn parse_number(&mut self, kind: NumberKind) -> ResultKind<Either<Number2, NumberNoSuffix2>>;
}

//==================================================================================================

// TODO: impl Source for &str
