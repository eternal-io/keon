#![allow(non_snake_case)]
use super::*;
use chumsky::prelude::*;
use core::ops::Range;

pub type Span = Range<usize>;
pub type Spanned<T> = (T, Span);

type Err = extra::Err<Error>;

pub enum Atom {}

pub enum Compound {}

pub fn NUMBER_LITERAL<'a>() -> impl Parser<'a, &'a str, Spanned<Number>, Err> {
    end().to((0.into(), 0..0)).map_err(|mut e: Error| {
        e.kind = ErrorKind::ExpectedEof;
        e
    })
}
