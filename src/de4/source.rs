use super::*;

pub(crate) enum Indicator<'de> {
    Unit,
    Bool(bool),
    Char(char),
    Byte(u8),
    String(StringKind),
    Bytes(BytesKind),
    Number(NumberKind),
    Initiator(Initiator),
    NominalPath(NominalPathRef<'de>),
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Initiator {
    /** `?` */ Maybe,
    /** `[` */ Array,
    /** `(` */ Tuple,
    /** `{` */ MapLike,
}

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delimiter {
    /** `]` */ Array,
    /** `)` */ Tuple,
    /** `}` */ MapLike,
    /** `,` */ Comma,
    /** `:` */ Colon,
    /** `=>`*/ FatArrow,
    /** `;` */ SemiColon,
               EOF,
}

pub(crate) enum NumberKind {
    Normal,
    Infinity,
    NegInfinity,
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

#[expect(private_bounds)]
pub trait Source<'de>: ParseToConcr<'de> + ParseToValue<'de> {}

pub(crate) trait ParseHelper<'de> {
    fn set_position(&mut self);

    /// Position of the most recent call to `set_position()`.
    fn position(&self) -> Position;

    /// Consumes the subsequent whitespaces and comments.
    fn eat_ws(&mut self) -> ResultKind;

    /// Skips WS and consumes the specified delimiter if possible:
    /// - returns `None` on success;
    /// - returns `Some` if another delimiter is found;
    /// - returns `Err` if no delimiter is found.
    fn delim(&mut self, delim: Delimiter) -> ResultKind<Option<Delimiter>>;

    /// Skips WS and consumes the specified delimiter. Returns `Err` if not found.
    fn delim_expected(&mut self, delim: Delimiter, reason: ErrorKind) -> ResultKind {
        if self.delim(delim)?.is_none() {
            Ok(())
        } else {
            Err(reason)
        }
    }

    /// Skips WS and checks the presence of a subsequent delimiter.
    fn adjacent_to_delim(&mut self) -> ResultKind<bool>;

    /// Skips WS and checks the presence of a subsequent delimiter. Returns `Err(reason)` if not found.
    fn adjacent_to_delim_expected(&mut self, reason: ErrorKind) -> ResultKind {
        self.adjacent_to_delim()?.then_some(()).ok_or(reason)
    }
}

pub(crate) trait ParseToConcr<'de>: ParseHelper<'de> {
    /// If the subsequent content starts with `b'`, consume it and return true.
    fn try_byte(&mut self) -> ResultKind<bool>;

    fn begin_char(&mut self) -> ResultKind;

    fn begin_string(&mut self) -> ResultKind<StringKind>;

    fn begin_bytes(&mut self) -> ResultKind<BytesKind>;

    fn begin_maybe(&mut self) -> ResultKind;

    fn begin_array(&mut self) -> ResultKind;
    fn end_array(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::Array)? {
            self.set_position();
            if let Delimiter::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedArrayEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_tuple(&mut self) -> ResultKind;
    fn end_tuple(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::Tuple)? {
            self.set_position();
            if let Delimiter::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedTupleEnd)
            }
        } else {
            Ok(())
        }
    }

    fn begin_map_like(&mut self) -> ResultKind;
    fn end_map_like(&mut self) -> ResultKind {
        if let Some(delim) = self.delim(Delimiter::MapLike)? {
            self.set_position();
            if let Delimiter::Comma = delim {
                Err(ErrorKind::DuplicatedComma)
            } else {
                Err(ErrorKind::ExpectedMapLikeEnd)
            }
        } else {
            Ok(())
        }
    }

    //------------------------------------------------------------------------------

    fn parse_unit(&mut self) -> ResultKind;

    fn parse_bool(&mut self) -> ResultKind<bool>;

    fn parse_byte(&mut self) -> ResultKind<u8>;

    // TODO: According to the grammar spec, WS is not allowed between negative signs and digits.
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

    // TODO: According to the grammar spec, WS is not allowed surrounding path separators.
    fn parse_nominal_path<'t>(&mut self, buf: &'t mut Vec<u8>) -> ResultKind<NominalPathRef<'t>>
    where
        'de: 't;

    fn parse_identifier<'t>(&mut self, buf: &'t mut Vec<u8>) -> ResultKind<IdentRef<'t>>
    where
        'de: 't;
}

pub(crate) trait ParseToValue<'de>: ParseHelper<'de> {
    fn begin(&mut self) -> ResultKind<Indicator<'de>>;

    /// Skips WS and consumes the subsequent initiator. Returns `None` if not found.
    fn initiator(&mut self) -> ResultKind<Option<Initiator>>;

    fn parse_number(&mut self, kind: NumberKind) -> ResultKind<Either<Number2, NumberNoSuffix2>>;
}

//==================================================================================================

// TODO: impl Source for &str
