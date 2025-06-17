use std::ops::Range;

use super::*;
use chumsky::prelude::*;

pub(crate) type Span = Range<usize>;
pub(crate) type Spanned<T> = (T, Span);

pub(crate) enum Atom {}

pub(crate) enum Compound {}

#[allow(non_snake_case)]
pub(crate) fn NUMBER_LITERAL<'i, E>() -> impl Parser<'i, &'i str, Spanned<Number>> {
    end().to((0.into(), 0..0))
}
