use self::error::*;
use crate::{format::*, value::*};
use core::{cmp::Ordering, marker::PhantomData};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use either::Either;

mod de_to_concr;
mod de_to_value;
pub mod error;

pub struct Deserializer<'de> {
    src: &'de str,
    pos: usize,
    ttl: isize,
    buf: Vec<u8>,
    stack: Vec<PunctStart>,
}

impl<'de> Deserializer<'de> {
    pub fn new(src: &'de str) -> Self {
        todo!()
    }

    pub fn parse<T>(&mut self) -> Result<T> {
        if self.ttl < 0 {
            return Err(todo!("previous errored"));
        }

        todo!()
    }
}

//==================================================================================================

enum Indicator<'de> {
    Unit,
    Bool(bool),
    Char(char),
    Byte(u8),
    Bytes(BytesKind),
    Number(NumberKind),
    String(StringKind),
    Maybe,
    PunctStart(PunctStart),
    NominalPath(NominalPathRef<'de>),
}

#[rustfmt::skip]
enum PunctStart {
    /** `(` */ Paren,
    /** `[` */ Brack,
    /** `{` */ Brace,
}

#[rustfmt::skip]
enum PunctDelim {
    /** `)` */ Paren,
    /** `]` */ Brack,
    /** `}` */ Brace,
    /** `,` */ Comma,
    /** `=>`*/ FatArrow,
    /** `;` */ Semicolon,
}

enum NumberKind {
    Number,
    Infinity,
    NegInfinity,
    NotANumber,
}

enum StringKind {
    Normal,
    RawString(usize),
    Paragraph(usize),
}

enum BytesKind {
    Normal,
    Raw(usize),
    Base64,
    Base32,
    Base16,
}

impl<'de> Deserializer<'de> {
    fn begin(&mut self) -> Result<Indicator<'de>> {
        if let Ok(kind) = self.begin_number() {
            match kind {
                Either::Left(kind) => Ok(Indicator::Number(kind)),
                Either::Right(byte) => Ok(Indicator::Byte(byte)),
            }
        } else if let Ok(kind) = self.begin_string() {
            Ok(Indicator::String(kind))
        } else {
            Err(todo!("unexp token"))
        }
    }

    /// Only [`NumberKind::Number`] does not consume leading content. All others do.
    fn begin_number(&mut self) -> Result<Either<NumberKind, u8>> {
        todo!()
    }

    fn begin_string(&mut self) -> Result<StringKind> {
        todo!()
    }

    fn begin_bytes(&mut self) -> Result<BytesKind> {
        todo!()
    }
}
