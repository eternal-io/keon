use super::*;
use crate::Sealed;

pub(crate) enum Indicator<'de> {
    Unit,
    Bool(bool),
    Char(char),
    Number(NumberKind),
    String(StringKind),
    Bytes(BytesKind),
    PunctStart(PunctStart),
    NominalPath(NominalPathRef<'de>),
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PunctStart {
    /** `?` */ Quest,
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
            PunctStart::Quest => todo!(),
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

#[expect(private_bounds, reason = "Sealed")]
pub trait Source<'de>: ReadConcr<'de> + ReadAny<'de> {}

pub(crate) trait Read<'de> {
    fn set_position(&mut self);

    /// Position of the most recent call to `set_position()`.
    fn position(&self) -> Position;

    /// Consumes the leading whitespace and comments.
    fn eat_ws(&mut self) -> ResultKind;

    /// Skips WS and consumes the specified start punctuation. Returns `Err` if not found.
    fn next_start(&mut self, start: PunctStart, reason: ErrorKind) -> ResultKind;

    /// Skips WS and consumes the specified delim punctuation. Returns `Err` if not found.
    fn next_delim(&mut self, delim: PunctDelim, reason: ErrorKind) -> ResultKind;

    /// Skips WS and consumes the specified delim punctuation if possible.
    /// Returns `None` on success; returns `Some(_)` if something else is found.
    fn try_next_delim(&mut self, delim: PunctDelim) -> ResultKind<Option<PunctDelim>>;

    /// Skips WS and peeks the subsequent delim punctuation. Returns `None` if not found.
    fn seek_delim(&mut self) -> ResultKind<Option<PunctDelim>>;

    /// Skips WS and peeks the subsequent delim punctuation. Returns `Err` if not found.
    fn seek_delim_expected(&mut self) -> ResultKind<PunctDelim> {
        self.seek_delim()?.ok_or(ErrorKind::ExpectedDelimiter)
    }
}

pub(crate) trait ReadConcr<'de>: Read<'de> {
    /// If the subsequent content starts with `b'`, consume it and return true.
    fn try_byte(&mut self) -> bool;

    fn begin_char(&mut self) -> ResultKind;

    fn begin_string(&mut self) -> ResultKind<StringKind>;

    fn begin_bytes(&mut self) -> ResultKind<BytesKind>;

    fn begin_maybe(&mut self) -> ResultKind {
        self.next_start(PunctStart::Quest, ErrorKind::ExpectedMaybe)
    }

    fn begin_tuple(&mut self) -> ResultKind {
        self.next_start(PunctStart::Paren, ErrorKind::ExpectedTuple)
    }
    fn end_tuple(&mut self) -> ResultKind {
        if let Some(delim) = self.try_next_delim(PunctDelim::Paren)? {
            self.set_position();
            if let PunctDelim::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedTupleEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_array(&mut self) -> ResultKind {
        self.next_start(PunctStart::Brack, ErrorKind::ExpectedArray)
    }
    fn end_array(&mut self) -> ResultKind {
        if let Some(delim) = self.try_next_delim(PunctDelim::Brack)? {
            self.set_position();
            if let PunctDelim::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedArrayEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_map_like(&mut self) -> ResultKind {
        self.next_start(PunctStart::Brace, ErrorKind::ExpectedMapLike)
    }
    fn end_map_like(&mut self) -> ResultKind {
        if let Some(delim) = self.try_next_delim(PunctDelim::Brace)? {
            self.set_position();
            if let PunctDelim::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedMapLikeEnd)
            }
        } else {
            Ok(())
        }
    }

    //------------------------------------------------------------------------------

    fn parse_unit(&mut self) -> ResultKind {
        self.next_start(PunctStart::Paren, ErrorKind::ExpectedUnit)?;
        self.next_delim(PunctDelim::Paren, ErrorKind::ExpectedUnitEnd)?;
        Ok(())
    }

    fn parse_bool(&mut self) -> ResultKind<bool>;

    fn parse_byte(&mut self) -> ResultKind<u8>;

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

    fn parse_char(&mut self) -> ResultKind<char>;

    fn parse_string<'t>(&mut self, kind: StringKind, buf: &'t mut Vec<u8>) -> ResultKind<Either<&'de str, &'t str>>;

    fn parse_bytes<'t>(&mut self, kind: BytesKind, buf: &'t mut Vec<u8>) -> ResultKind<Either<&'de [u8], &'t [u8]>>;

    fn parse_identifier<'t>(&mut self, buf: &'t mut Vec<u8>) -> ResultKind<&'t str>
    where
        'de: 't;

    fn parse_nominal_path<'t>(&mut self, buf: &'t mut Vec<u8>) -> ResultKind<NominalPathRef<'t>>
    where
        'de: 't;
}

pub(crate) trait ReadAny<'de>: Read<'de> {
    fn begin(&mut self) -> ResultKind<Indicator<'de>>;

    fn parse_number(&mut self, kind: NumberKind) -> ResultKind<Either<Number2, NumberNoSuffix2>>;
}

//==================================================================================================

// TODO: impl Source for &str
