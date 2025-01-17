use super::{lexer::*, *};
use logos::{Lexer, Logos};
use serde::de::{
    value::{EnumAccessDeserializer, StrDeserializer},
    DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use smol_str::SmolStr;
use std::{iter::Peekable, num::NonZeroU32};

/// Conveniently get `T` from deserialize a str.
pub fn from_str<'de, T: serde::Deserialize<'de>>(s: &'de str) -> Result<T> {
    let mut der = Deserializer::from_str(s);
    let val = T::deserialize(&mut der)?;
    der.end()?;
    Ok(val)
}

//==================================================================================================

struct Kexer<'i> {
    lex: Lexer<'i, Token<'i>>,
}

impl<'i> Kexer<'i> {
    fn from_str(s: &'i str) -> Self {
        Self { lex: Token::lexer(s) }
    }
}

impl<'i> Iterator for Kexer<'i> {
    type Item = Result<(Token<'i>, Location)>;
    fn next(&mut self) -> Option<Self::Item> {
        self.lex.next().map(|res| {
            let InnerExtras { line, line_start } = *self.lex.extras.borrow();
            let location = Location {
                line,
                line_start,
                token_start: self.lex.span().start,
            };

            match res {
                Ok(t) => Ok((t, location)),
                Err(e) => Err(location.locate(self.lex.source(), e)),
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Location {
    line: u32,
    line_start: usize,
    token_start: usize,
}

impl Location {
    fn locate(&self, src: &str, kind: ErrorKind) -> Error {
        let Location {
            line,
            line_start,
            token_start,
        } = self;
        let col = match line_start > token_start {
            true => None, // we meet unexpected newline.
            false => Some(src[*line_start..*token_start].chars().count() as u32 + 1),
        };
        Error {
            line: Some(NonZeroU32::new(line + 1).unwrap()),
            col: col.map(|n| NonZeroU32::new(n).unwrap()),
            kind,
        }
    }

    fn raise<T>(&self, src: &str, kind: ErrorKind) -> Result<T> {
        Err(self.locate(src, kind))
    }
}

macro_rules! unwrap_ident {
    ($expr:expr) => {{
        let Token::Ident(name) = $expr else { unreachable!() };
        SmolStr::new(name)
    }};
}

//==================================================================================================

/// The KEON deserializer.
///
/// Usually convenience function [`from_str`] is enough.
pub struct Deserializer<'de> {
    src: &'de str,
    lex: Peekable<Kexer<'de>>,
    ttl: usize,
}

impl<'de> Deserializer<'de> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(src: &'de str) -> Self {
        Self {
            src,
            lex: Kexer::from_str(src).peekable(),
            ttl: RECURSION_LIMIT,
        }
    }

    /// Checks whether the remaining characters are only whitespaces, returns an error if don't.
    pub fn end(&mut self) -> Result<()> {
        if let Some((_, loc, src)) = self.next()? {
            loc.raise(src, ErrorKind::ExpectedEof)?
        }

        Ok(())
    }

    /// To bypass the borrow checker, the extra `&str` is necessary.
    fn next(&mut self) -> Result<Option<(Token, Location, &str)>> {
        Ok(self.lex.next().transpose()?.map(|(t, loc)| (t, loc, self.src)))
    }

    fn peek(&mut self) -> Result<Option<(TokenKind, Location, &str)>> {
        Ok(match self.lex.peek() {
            None => None,
            Some(res) => match res {
                Ok((t, loc)) => Some((t.kind(), *loc, self.src)),
                Err(e) => Err(e.clone())?,
            },
        })
    }

    fn expect_next(&mut self) -> Result<(Token, Location, &str)> {
        self.next()
            .and_then(|opt| opt.ok_or(Error::new(ErrorKind::UnexpectedEof)))
    }

    fn expect_peek(&mut self) -> Result<(TokenKind, Location, &str)> {
        self.peek()
            .and_then(|opt| opt.ok_or(Error::new(ErrorKind::UnexpectedEof)))
    }

    fn expect_consume_token(&mut self, token_kind: TokenKind, error_kind: ErrorKind) -> Result<Token> {
        self.next()?
            .ok_or(Error::new(ErrorKind::UnexpectedEof))
            .and_then(|(t, loc, src)| {
                (t.kind() == token_kind)
                    .then_some(t)
                    .ok_or_else(|| loc.locate(src, error_kind))
            })
    }

    fn try_consume_token(&mut self, token_kind: TokenKind) -> Result<Option<Token>> {
        Ok(match self.peek()? {
            None => None,
            Some((tk, ..)) => match tk == token_kind {
                false => None,
                true => Some(self.next().unwrap().unwrap().0),
            },
        })
    }
}

impl<'de> serde::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        let (ttl, overflow) = self.ttl.overflowing_sub(1);
        self.ttl = ttl;

        let (t, loc, src) = self.expect_next()?;
        if overflow {
            loc.raise(src, ErrorKind::ExceededRecursionLimit)?
        }

        let val = match t {
            Token::Literal(literal) => parse_literal(literal, vis),
            Token::Question => parse_option(self, vis),
            Token::Paren_ => parse_parenthesis(self, vis),
            Token::Brack_ => parse_seq(self, vis),
            Token::Brace_ => parse_map(self, vis),
            Token::Percent => parse_mayary(self, vis),
            Token::Ident(ident) => {
                let name = SmolStr::new(ident);
                parse_enum(self, vis, name)
            }
            _ => loc.raise(src, ErrorKind::UnexpectedToken),
        }?;

        self.ttl += 1;
        Ok(val)
    }
}

fn parse_literal<'de, V: Visitor<'de>>(literal: Literal, vis: V) -> Result<V::Value> {
    match literal {
        Literal::Bool(b) => vis.visit_bool(b),
        Literal::Int(i) => vis.visit_i64(i),
        Literal::UInt(u) => vis.visit_u64(u),
        Literal::Float(f) => vis.visit_f64(f),
        Literal::Char(ch) => vis.visit_char(ch),
        Literal::Str(s) => vis.visit_str(s),
        Literal::String(s) => vis.visit_string(s),
        Literal::Bytes(bytes) => vis.visit_bytes(bytes),
        Literal::ByteBuf(buf) => vis.visit_byte_buf(buf),
    }
}

/// Requires the leading question mark `?` has been consumed.
///
/// - None: `?`.
/// - Some: `? Thing`.
fn parse_option<'i, 'de, V: Visitor<'de>>(der: &'i mut Deserializer<'de>, vis: V) -> Result<V::Value> {
    match der.peek()? {
        None => vis.visit_none(),
        Some((tk, ..)) => match tk.is_delimiter() {
            true => vis.visit_none(),
            false => vis.visit_some(der),
        },
    }
}

/// Requires the leading percentage `%` has been consumed.
///
/// A mayary, "maybe-ary", equivalent to a tuple with zero or one item.
///
/// - Nullary: `%`, a rare case, but Serde does support it.
/// - Unary: `% T`, also known as "newtype".
///
/// Usage is like option: `%` and `% Thing`.
fn parse_mayary<'i, 'de, V: Visitor<'de>>(der: &'i mut Deserializer<'de>, vis: V) -> Result<V::Value> {
    match der.peek()? {
        None => vis.visit_seq(NullaryAccessor),
        Some((tk, ..)) => match tk.is_delimiter() {
            true => vis.visit_seq(NullaryAccessor),
            false => vis.visit_newtype_struct(der),
        },
    }
}

/// Requires the leading parenthesis `(` has been consumed.
///
/// - Unit: `()` or `(MyStruct)` optional struct name just like type conversion in C.
/// - Tuple: `(T, U, V, ...)`.
/// - Unary tuple: `(T,)`.
///
/// and the following notable representations:
///
/// - Nullary tuple: `(AwfulNullary)()` or simply `()()`.
/// - Alt unary tuple: `(CommonNewtype)(T)` or simply `()(T)`.
fn parse_parenthesis<'i, 'de, V: Visitor<'de>>(der: &'i mut Deserializer<'de>, vis: V) -> Result<V::Value> {
    match der.expect_peek()?.0 {
        TokenKind::_Paren => {
            der.next().ok();
        }
        TokenKind::Ident => {
            let mut name = unwrap_ident!(der.next().unwrap().unwrap().0);
            match der.expect_peek()?.0 {
                TokenKind::_Paren => {
                    der.next().ok();
                }
                TokenKind::PathSep => {
                    der.next().ok();
                    name = unwrap_ident!(der.expect_consume_token(TokenKind::Ident, ErrorKind::ExpectedVariant)?);
                    return parse_tuple_alt(der, vis, name);
                }
                _ => return parse_tuple_alt(der, vis, name),
            }
        }
        _ => return parse_tuple::<_, false>(der, vis),
    }

    match der.peek()? {
        None => vis.visit_unit(),
        Some((tk, loc, src)) => match tk {
            TokenKind::Paren_ => {
                der.next().ok();
                parse_tuple::<_, true>(der, vis)
            }
            TokenKind::Brace_ => {
                der.next().ok();
                parse_map(der, vis)
            }
            TokenKind::Percent => {
                der.next().ok();
                vis.visit_newtype_struct(der)
            }
            _ if tk.is_delimiter() => vis.visit_unit(),
            _ => loc.raise(src, ErrorKind::ExpectedNonUnitStruct),
        },
    }
}

/// Requires the leading parenthesis `(` has been consumed.
///
/// - Tuple: `(T,)`, `(T, U, V, ...)`.
/// - Docile tuple: `()` and `(Name)` are both legal.
fn parse_tuple<'i, 'de, V: Visitor<'de>, const DOCILE: bool>(
    der: &'i mut Deserializer<'de>,
    vis: V,
) -> Result<V::Value> {
    vis.visit_seq(TupleAccessor::new::<DOCILE>(der)?)
}

/// Requires the leading `(` `Enum::Variant` has been consumed, and the `Variant` must be provided in parameter.
///
/// - Tuple starts with variant: `(Enum::Variant,)`, `(Variant,)` or `(Variant, ...)`.
fn parse_tuple_alt<'i, 'de, V: Visitor<'de>>(
    der: &'i mut Deserializer<'de>,
    vis: V,
    variant: SmolStr,
) -> Result<V::Value> {
    vis.visit_seq(TupleAccessor::with_first_variant::<false>(der, variant)?)
}

/// Requires the leading bracket `[` has been consumed.
///
/// - Sequence: `[0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]`.
fn parse_seq<'i, 'de, V: Visitor<'de>>(der: &'i mut Deserializer<'de>, vis: V) -> Result<V::Value> {
    vis.visit_seq(SeqAccessor::new(der)?)
}

/// Requires the leading brace `{` has been consumed.
///
/// - Map-like: `{ 1 => 2, 3 => 4 }`.
/// - Struct-like: `{ name: "Alex", age: 31 }`.
///
/// These two can be mixed, but be careful with delimiters:
///
/// - If `=>` was used, key can be deserialized into arbitrary type
///   (to represent a string, put `"key"`)
/// - If `:` was used, key will and will only be deserialized into a string
///   (safe to use as a field name at this point)
fn parse_map<'i, 'de, V: Visitor<'de>>(der: &'i mut Deserializer<'de>, vis: V) -> Result<V::Value> {
    vis.visit_map(MapAccessor::new(der)?)
}

/// The leading identifier must be provided in parameter.
///
/// - Nameness: `Difficulty::Easy`.
/// - Nameless: `Medium`, `Hard { heart: 1 }`.
fn parse_enum<'i, 'de, V: Visitor<'de>>(der: &'i mut Deserializer<'de>, vis: V, mut name: SmolStr) -> Result<V::Value> {
    if der.try_consume_token(TokenKind::PathSep)?.is_some() {
        name = unwrap_ident!(der.expect_consume_token(TokenKind::Ident, ErrorKind::ExpectedVariant)?);
    }

    vis.visit_enum(EnumAccessor::new(der, name))
}

//==================================================================================================

struct NullaryAccessor;
impl<'de> SeqAccess<'de> for NullaryAccessor {
    type Error = Error;

    fn size_hint(&self) -> Option<usize> {
        Some(0)
    }
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, _seed: T) -> Result<Option<T::Value>> {
        Ok(None)
    }
}

struct TupleAccessor<'i, 'de> {
    der: &'i mut Deserializer<'de>,
    yielding: bool,
    first_variant: Option<SmolStr>,

    /// Once this value equals to `1`, it's expected a comma `,` before closing.
    ///
    /// This value will be increased after each `next_element_seed` call.
    ctr: u32,
}
impl<'i, 'de> TupleAccessor<'i, 'de> {
    /// Requires the leading parenthesis `(` has been consumed.
    fn new<const DOCILE: bool>(der: &'i mut Deserializer<'de>) -> Result<Self> {
        Self::_build::<DOCILE>(der, None)
    }

    /// Requires the leading `(` `Enum::Variant` has been consumed, and the `Variant` must be provided in parameter.
    fn with_first_variant<const DOCILE: bool>(der: &'i mut Deserializer<'de>, first_variant: SmolStr) -> Result<Self> {
        Self::_build::<DOCILE>(der, Some(first_variant))
    }

    fn _build<const DOCILE: bool>(der: &'i mut Deserializer<'de>, first_variant: Option<SmolStr>) -> Result<Self> {
        Ok(Self {
            first_variant,
            yielding: der.try_consume_token(TokenKind::_Paren)?.is_none(),
            ctr: DOCILE.into(),
            der,
        })
    }
}
impl<'de> SeqAccess<'de> for TupleAccessor<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if !self.yielding {
            return Ok(None);
        }

        let val = match self.first_variant.take() {
            None => seed.deserialize(&mut *self.der)?,
            Some(variant) => {
                seed.deserialize(EnumAccessDeserializer::new(EnumAccessor::new(&mut *self.der, variant)))?
            }
        };

        self.ctr += 1;

        match self.der.try_consume_token(TokenKind::Comma)? {
            Some(_) => self.yielding = self.der.try_consume_token(TokenKind::_Paren)?.is_none(),
            None => {
                let (tk, loc, src) = self.der.expect_peek()?;
                match tk {
                    TokenKind::_Paren if self.ctr != 1 => {
                        self.der.next().ok();
                        self.yielding = false;
                    }
                    _ => loc.raise(src, ErrorKind::ExpectedComma)?,
                }
            }
        }

        Ok(Some(val))
    }
}

struct SeqAccessor<'i, 'de> {
    der: &'i mut Deserializer<'de>,
    yielding: bool,
}
impl<'i, 'de> SeqAccessor<'i, 'de> {
    /// Requires the leading bracket `[` has been consumed.
    fn new(der: &'i mut Deserializer<'de>) -> Result<Self> {
        Ok(Self {
            yielding: der.try_consume_token(TokenKind::_Brack)?.is_none(),
            der,
        })
    }
}
impl<'de> SeqAccess<'de> for SeqAccessor<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if !self.yielding {
            return Ok(None);
        }

        let val = seed.deserialize(&mut *self.der)?;

        match self.der.try_consume_token(TokenKind::Comma)? {
            Some(_) => self.yielding = self.der.try_consume_token(TokenKind::_Brack)?.is_none(),
            None => {
                self.der
                    .expect_consume_token(TokenKind::_Brack, ErrorKind::ExpectedComma)?;
                self.yielding = false;
            }
        }

        Ok(Some(val))
    }
}

struct MapAccessor<'i, 'de> {
    der: &'i mut Deserializer<'de>,
    yielding: bool,
}
impl<'i, 'de> MapAccessor<'i, 'de> {
    /// Requires the leading brace `{` has been consumed.
    fn new(der: &'i mut Deserializer<'de>) -> Result<Self> {
        Ok(Self {
            yielding: der.try_consume_token(TokenKind::_Brace)?.is_none(),
            der,
        })
    }
}
impl<'de> MapAccess<'de> for MapAccessor<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if !self.yielding {
            return Ok(None);
        }

        let val;

        match self.der.try_consume_token(TokenKind::Ident)? {
            None => {
                /* Arbitrary => */
                val = seed.deserialize(&mut *self.der)?;

                self.der
                    .expect_consume_token(TokenKind::FatArrow, ErrorKind::ExpectedFatArrow)?;
            }
            Some(ident) => {
                /* Field or Enum::Variant */
                let mut name = unwrap_ident!(ident);
                match self.der.try_consume_token(TokenKind::Colon)? {
                    Some(_) => {
                        /* Field: */
                        val = seed.deserialize(StrDeserializer::<Error>::new(&name))?;
                    }
                    None => {
                        /* Enum::Variant => */
                        if self.der.try_consume_token(TokenKind::PathSep)?.is_some() {
                            name = unwrap_ident!(self
                                .der
                                .expect_consume_token(TokenKind::Ident, ErrorKind::ExpectedVariant)?);
                        }

                        val = seed.deserialize(EnumAccessDeserializer::new(EnumAccessor::new(&mut *self.der, name)))?;

                        self.der
                            .expect_consume_token(TokenKind::FatArrow, ErrorKind::ExpectedFatArrow)?;
                    }
                }
            }
        }

        Ok(Some(val))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let val = seed.deserialize(&mut *self.der)?;

        match self.der.try_consume_token(TokenKind::Comma)? {
            Some(_) => self.yielding = self.der.try_consume_token(TokenKind::_Brace)?.is_none(),
            None => {
                self.der
                    .expect_consume_token(TokenKind::_Brace, ErrorKind::ExpectedComma)?;
                self.yielding = false;
            }
        }

        Ok(val)
    }
}

struct EnumAccessor<'i, 'de> {
    der: &'i mut Deserializer<'de>,
    variant: SmolStr,
}
impl<'i, 'de> EnumAccessor<'i, 'de> {
    /// Requires the leading `Enum::Variant` has been consumed, and the `Variant` must be provided in parameter.
    fn new(der: &'i mut Deserializer<'de>, variant: SmolStr) -> Self {
        Self { der, variant }
    }
}
impl<'i, 'de> EnumAccess<'de> for EnumAccessor<'i, 'de> {
    type Error = Error;
    type Variant = VariantAccessor<'i, 'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        Ok((
            seed.deserialize(StrDeserializer::<Error>::new(&self.variant))?,
            VariantAccessor::new(self.der),
        ))
    }
}

struct VariantAccessor<'i, 'de> {
    der: &'i mut Deserializer<'de>,
}
impl<'i, 'de> VariantAccessor<'i, 'de> {
    fn new(der: &'i mut Deserializer<'de>) -> Self {
        Self { der }
    }
}
impl<'de> VariantAccess<'de> for VariantAccessor<'_, 'de> {
    type Error = Error;

    /// Note that inputs like `Variant()` is a nullary tuple variant instead.
    fn unit_variant(self) -> Result<()> {
        if let Some((tk, loc, src)) = self.der.peek()? {
            if !tk.is_delimiter() {
                loc.raise(src, ErrorKind::ExpectedUnitVariant)?
            }
        }

        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        let (tk, loc, src) = self.der.expect_next()?;
        match tk {
            Token::Percent => seed.deserialize(&mut *self.der),
            Token::Paren_ => {
                let val = seed.deserialize(&mut *self.der)?;
                self.der.try_consume_token(TokenKind::Comma)?;
                self.der
                    .expect_consume_token(TokenKind::_Paren, ErrorKind::ExpectedNewtypeVariant)?;
                Ok(val)
            }
            _ => loc.raise(src, ErrorKind::ExpectedNewtypeVariant),
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _: usize, vis: V) -> Result<V::Value> {
        self.der
            .expect_consume_token(TokenKind::Paren_, ErrorKind::ExpectedTupleVariant)?;

        parse_tuple::<_, true>(self.der, vis)
    }

    fn struct_variant<V: Visitor<'de>>(self, _: &'static [&'static str], vis: V) -> Result<V::Value> {
        self.der
            .expect_consume_token(TokenKind::Brace_, ErrorKind::ExpectedStructVariant)?;

        parse_map(self.der, vis)
    }
}
