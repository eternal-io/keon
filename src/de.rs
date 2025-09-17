use super::{value::*, *};
use core::{cmp::Ordering, marker::PhantomData, num::NonZeroU8};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use lexical_core::{
    NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions, ParseIntegerOptionsBuilder,
};
use serde::Deserialize;

pub mod de_to_concr;
pub mod de_to_value;

pub fn parse<'de, T>(s: &'de str) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut der = Parser::new(s);
    let value = T::deserialize(&mut der)?;
    der.finish().and(Ok(value))
}

pub fn parse_many<'de, T>(s: &'de str) -> IterParser<'de, T>
where
    T: Deserialize<'de>,
{
    Parser::new(s).into_iter()
}

//------------------------------------------------------------------------------

pub struct IterParser<'de, T> {
    der: Parser<'de>,
    phantom: PhantomData<T>,
}

impl<'de, T> IterParser<'de, T>
where
    T: Deserialize<'de>,
{
    #[inline]
    pub fn into_inner(self) -> Parser<'de> {
        self.der
    }

    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.der.has_reached_end()
    }

    #[inline]
    pub fn is_corrupted(&self) -> bool {
        self.der.is_corrupted()
    }
}

impl<'de, T> Iterator for IterParser<'de, T>
where
    T: Deserialize<'de>,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.der.is_corrupted() {
            return Some(self.der.raise(ErrorKind::Corrupted));
        }
        if self.der.has_reached_end() {
            return None;
        }

        let e = 'fail: {
            if let Err(e) = self.der.consume_whitespace_comment_first() {
                break 'fail e;
            }
            let v = match T::deserialize(&mut self.der) {
                Err(e) => break 'fail e,
                Ok(v) => v,
            };
            if let Err(e) = self.der.finish_one() {
                break 'fail e;
            }

            return Some(Ok(v));
        };

        Some(Err(e))
    }
}

//------------------------------------------------------------------------------

#[doc(alias = "Deserializer")]
pub struct Parser<'de> {
    src: &'de str,
    pos: usize,

    /// To avoid possible failures when creating a parser,
    /// whitespace handling is moved to the first value being parsed.
    /// However, only the first value requires additional whitespace handling,
    /// as [`consume_ws_(";")`](Self::consume_ws_) handles all leading whitespace after the first value.
    /// Therefore, this flag exists to allow for some simple optimizations.
    ///
    /// Implementations can simply call [`Self::consume_whitespace_comment_first`] before parsing each value to simplify the work.
    leading_ws_handled: bool,

    /// If a parser has previously failed, to prevent it from being used again,
    /// all subsequent uses of failable methods will fail with [`ErrorKind::Corrupted`].
    ///
    /// This flag is primarily maintained by `raise_` methods. Implementations should not
    /// forget to set this flag if the [`Result`] is not constructed via a parser (e.g.
    /// via [`serde::de::Visitor`]). The convenience method [`Self::watch`] can be useful.
    ///
    /// All non-trait methods on this parser are guaranteed to handle this flag correctly.
    corrupted: bool,
}

impl<'de> Parser<'de> {
    #[inline]
    pub const fn new(src: &'de str) -> Self {
        Self {
            src,
            pos: 0,
            leading_ws_handled: false,
            corrupted: false,
        }
    }

    #[inline]
    #[allow(clippy::should_implement_trait)]
    pub fn into_iter<T: Deserialize<'de>>(self) -> IterParser<'de, T> {
        IterParser {
            der: self,
            phantom: PhantomData,
        }
    }

    /// Returns `Ok(_)` if the current value is finished correctly and no more values.
    #[inline]
    pub fn finish(&mut self) -> Result<()> {
        self.corrupt_guard()?;
        self.consume_ws_(";")?;
        if self.has_reached_end() {
            Ok(())
        } else {
            self.raise(ErrorKind::ExpectedEnd)
        }
    }

    /// Returns `Ok(_)` if the current value is finished correctly.
    /// The `bool` inside indicates whether there are more values.
    #[inline]
    pub fn finish_one(&mut self) -> Result<bool> {
        self.corrupt_guard()?;
        if self.consume_ws_(";")? {
            Ok(!self.has_reached_end())
        } else if self.has_reached_end() {
            Ok(false)
        } else {
            self.raise(ErrorKind::ExpectedSemiOrEnd)
        }
    }

    #[inline]
    pub fn has_reached_end(&self) -> bool {
        self.rest().is_empty()
    }

    #[inline]
    pub fn is_corrupted(&self) -> bool {
        self.corrupted
    }
}

//------------------------------------------------------------------------------

const INTEGER_FORMAT: u128 = lexical_core::format::RUST_LITERAL;

const INTEGER_FORMAT_HEX: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .base_prefix(Some(NonZeroU8::new(b'x').unwrap()))
    .mantissa_radix(16)
    .build();
const INTEGER_FORMAT_OCT: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .base_prefix(Some(NonZeroU8::new(b'o').unwrap()))
    .mantissa_radix(8)
    .build();
const INTEGER_FORMAT_BIN: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .base_prefix(Some(NonZeroU8::new(b'b').unwrap()))
    .mantissa_radix(2)
    .build();

const PARSE_INTEGER_OPTS: ParseIntegerOptions = ParseIntegerOptionsBuilder::new()
    .no_multi_digit(false)
    .build_unchecked();

const FLOAT_FORMAT: u128 = NumberFormatBuilder::rebuild(INTEGER_FORMAT)
    .required_fraction_digits(false)
    .no_special(false)
    .build();

const PARSE_FLOAT_OPTS: ParseFloatOptions = ParseFloatOptionsBuilder::new()
    .lossy(false)
    .nan_string(Some(b"NaN"))
    .inf_string(Some(b"inf"))
    .infinity_string(None)
    .build_unchecked();

fn is_whitespace(ch: &char) -> bool {
    ch.is_whitespace()
}

fn is_backtick(byte: &u8) -> bool {
    *byte == b'`'
}

enum Token<'de> {
    Keyword(Keyword),
    Identifier(&'de str),
    Underscore,
}

enum Keyword {
    NotANumber,
    False,
    Infinity,
    Long,
    True,
}

/// The keyword list is sorted and must be sorted.
const KEYWORDS: &[&str] = &["NaN", "false", "inf", "long", "true"];

impl Into<&'static str> for Keyword {
    fn into(self) -> &'static str {
        KEYWORDS[self as usize]
    }
}

impl TryFrom<&str> for Keyword {
    type Error = ();

    fn try_from(s: &str) -> StdResult<Self, Self::Error> {
        KEYWORDS.binary_search(&s).or(Err(())).map(|idx| match idx {
            0 => Self::NotANumber,
            1 => Self::False,
            2 => Self::Infinity,
            3 => Self::Long,
            4 => Self::True,
            _ => unreachable!(),
        })
    }
}

impl<'de> Parser<'de> {
    #[inline]
    fn watch<T>(&mut self, res: Result<T>) -> Result<T> {
        if res.is_err() {
            self.corrupted = true;
        }
        res
    }

    #[inline]
    fn corrupt_guard(&mut self) -> Result<()> {
        if self.corrupted {
            self.raise(ErrorKind::Corrupted)
        } else {
            Ok(())
        }
    }

    #[inline]
    const fn rest(&self) -> &'de str {
        self.src.split_at(self.pos).1
    }
    #[inline]
    const fn rest_bytes(&self) -> &'de [u8] {
        self.source_bytes().split_at(self.pos).1
    }
    #[inline]
    const fn source_bytes(&self) -> &'de [u8] {
        self.src.as_bytes()
    }
    #[inline]
    const fn peek_byte(&self) -> Option<u8> {
        self.rest_bytes().first().copied()
    }
    #[inline]
    const fn adjacent_to_delim(&self) -> bool {
        matches!(
            self.rest_bytes(),
            [b'=', b'>', ..] | [b')', ..] | [b']', ..] | [b'}', ..] | [b',', ..] | [b';', ..] | []
        )
    }

    #[inline]
    const fn bump(&mut self, n: usize) -> &'de str {
        let delta = self.rest().split_at(n).0;
        self.pos += n;
        delta
    }
    #[inline]
    const fn bump_to_end(&mut self) -> &'de str {
        let delta = self.rest();
        self.pos = self.src.len();
        delta
    }
    #[inline]
    const fn try_bump(&mut self, n: usize) -> Option<&'de [u8]> {
        if self.src.is_char_boundary(self.pos + n) {
            let delta = self.rest_bytes().split_at(n).0;
            self.pos += n;
            Some(delta)
        } else {
            None
        }
    }

    #[inline]
    fn raise<T>(&mut self, kind: ErrorKind) -> Result<T> {
        self.raise_at(self.pos, kind)
    }
    #[inline]
    fn raise_at<T>(&mut self, pos: usize, kind: ErrorKind) -> Result<T> {
        self.corrupted = true;
        Error::raise_at(pos, kind)
    }
    #[inline]
    fn raise_unexpected_end<T>(&mut self) -> Result<T> {
        self.corrupted = true;
        Error::raise_at(self.src.len(), ErrorKind::UnexpectedEnd)
    }

    #[inline]
    fn consume(&mut self, pat: &'static str) -> bool {
        match self.rest().starts_with(pat) {
            false => false,
            true => {
                self.bump(pat.len());
                true
            }
        }
    }

    #[inline]
    fn consume_if(&mut self, pred: impl FnOnce(&char) -> bool) -> bool {
        match self.rest().chars().next().filter(pred) {
            None => false,
            Some(ch) => {
                self.bump(ch.len_utf8());
                true
            }
        }
    }

    #[inline]
    fn consume_ws_(&mut self, pat: &'static str) -> Result<bool> {
        match self.rest().starts_with(pat) {
            false => Ok(false),
            true => {
                self.bump(pat.len());
                self.consume_whitespace_comment()?;
                Ok(true)
            }
        }
    }

    #[inline]
    fn consume_while(&mut self, mut pred: impl FnMut(&char) -> bool) -> &'de [u8] {
        self.bump(
            self.rest()
                .char_indices()
                .take_while(|(_off, ch)| pred(ch))
                .last()
                .map(|(off, ch)| off + ch.len_utf8())
                .unwrap_or(0),
        )
        .as_bytes()
    }

    #[inline]
    fn consume_while_fast(&mut self, mut pred: impl FnMut(&u8) -> bool) -> &'de [u8] {
        self.bump(
            self.rest_bytes()
                .iter()
                .enumerate()
                .take_while(|(_off, byte)| pred(byte))
                .last()
                .map(|(off, _byte)| off + 1)
                .unwrap_or(0),
        )
        .as_bytes()
    }

    #[inline]
    fn consume_ident(&mut self) -> Result<&'de str> {
        let start = self.pos;
        match self.consume_keyword_or_ident_or_underscore()? {
            Token::Keyword(kw) => self.raise_at(start, ErrorKind::UnexpectedKeywordIdent { keyword: kw.into() }),

            Token::Identifier(ident) => Ok(ident),

            Token::Underscore => self.raise_at(start, ErrorKind::UnexpectedUnderscoreIdent),
        }
    }

    #[inline]
    fn consume_ident_or_underscore(&mut self) -> Result<Option<&'de str>> {
        let start = self.pos;
        match self.consume_keyword_or_ident_or_underscore()? {
            Token::Keyword(kw) => self.raise_at(start, ErrorKind::UnexpectedKeywordIdent { keyword: kw.into() }),

            Token::Identifier(ident) => Ok(Some(ident)),

            Token::Underscore => Ok(None),
        }
    }

    #[inline]
    fn consume_keyword_or_ident_or_underscore(&mut self) -> Result<Token<'de>> {
        let raw_mode = self.consume("`");
        let start = self.pos;
        let need_more = if self.consume("_") {
            true
        } else if self.consume_if(|ch| unicode_ident::is_xid_start(*ch)) {
            false
        } else {
            return self.raise_at(start, ErrorKind::ExpectedIdent);
        };

        let no_more = self.consume_while(|ch| unicode_ident::is_xid_continue(*ch)).is_empty();
        if need_more && no_more {
            return Ok(Token::Underscore);
        }

        let end = self.pos;

        self.consume_whitespace_comment()?;

        let ident = &self.src[start..end];
        let token = if !raw_mode {
            if let Ok(kw) = Keyword::try_from(ident) {
                Token::Keyword(kw)
            } else {
                Token::Identifier(ident)
            }
        } else {
            Token::Identifier(ident)
        };

        Ok(token)
    }

    #[inline]
    fn consume_nominal_path_of_struct(&mut self, name: &'static str) -> Result<bool> {
        let mut stem = self.consume_ident_or_underscore()?;
        if self.consume_ws_("::")? {
            stem = self.consume_ident_or_underscore()?;
        }

        Ok(stem.map(|s| s == name).unwrap_or(true))
    }

    #[inline]
    fn consume_nominal_path_of_enum(&mut self, name: &'static str) -> Result<Option<&'de str>> {
        let stem = self.consume_ident_or_underscore()?;
        if self.consume_ws_("::")? {
            let parent = stem;
            let stem = self.consume_ident()?;

            Ok(parent.map(|p| p == name).unwrap_or(true).then_some(stem))
        } else {
            Ok(stem)
        }
    }

    #[inline]
    fn consume_newline(&mut self) -> Result<()> {
        self.consume("\r");
        if !self.consume("\n") {
            self.raise(ErrorKind::UnexpectedCarriageReturn)
        } else {
            Ok(())
        }
    }

    #[inline]
    fn consume_whitespace_comment(&mut self) -> Result<()> {
        loop {
            self.consume_while(is_whitespace);

            if self.consume("//") {
                match memchr::memchr(b'\n', self.rest_bytes()) {
                    Some(off) => {
                        self.bump(off);
                    }
                    None => {
                        self.pos = self.src.len();
                        break;
                    }
                }
            } else if self.consume("/*") {
                let mut depth = 1u8;

                while depth != 0 {
                    let Some(off) = memchr::memchr(b'/', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    if let Some(b'*') = self.bump(off).as_bytes().last() {
                        self.bump(1);
                        depth -= 1;
                    } else if self.consume("/*") {
                        depth = match depth.checked_add(1) {
                            Some(n) => n,
                            None => return self.raise(ErrorKind::DeeplyNestedComment),
                        }
                    }
                }
            } else {
                break;
            }
        }

        self.consume_while(is_whitespace);

        Ok(())
    }

    #[inline]
    fn consume_whitespace_comment_first(&mut self) -> Result<()> {
        if !self.leading_ws_handled {
            self.consume_whitespace_comment()?;
            self.leading_ws_handled = true;
        }

        Ok(())
    }

    #[inline]
    fn __escape_common(&mut self) -> Result<u8> {
        if let Some(byte) = self.peek_byte().and_then(|byte| {
            Some(match byte {
                b'\\' => b'\\',
                b'\"' => b'\"',
                b'\'' => b'\'',
                b'0' => b'\0',
                b'n' => b'\n',
                b't' => b'\t',
                b'r' => b'\r',
                _ => return None,
            })
        }) {
            self.bump(1);
            Ok(byte)
        } else {
            self.raise(ErrorKind::InvalidEscape)
        }
    }

    #[inline]
    fn __escape_byte(&mut self) -> Option<Result<u8>> {
        if self.consume("x") {
            Some({
                if let Some(delta) = self.try_bump(2) {
                    if delta[0].is_ascii_hexdigit() && delta[1].is_ascii_hexdigit() {
                        return Some(Ok(lexical_core::parse::<u8>(delta).unwrap()));
                    }
                }
                self.raise(ErrorKind::InvalidByteEscape)
            })
        } else {
            None
        }
    }

    #[inline]
    fn __escape_char(&mut self) -> Option<Result<char>> {
        if self.consume("x") {
            Some({
                if let Some(delta) = self.try_bump(2) {
                    if matches!(delta[0], b'0'..=b'7') && delta[1].is_ascii_hexdigit() {
                        return Some(Ok(lexical_core::parse::<u8>(delta).unwrap().into()));
                    }
                }
                self.raise(ErrorKind::InvalidAsciiEscape)
            })
        } else {
            self.consume("u").then(|| {
                let start = self.pos;
                let delta = self.consume_while(|ch| *ch != '}');
                let Some((b'{', delta)) = delta.split_first() else {
                    return self.raise_at(start, ErrorKind::Expected("`{`"));
                };

                let chr = lexical_core::parse_with_options::<
                    u32,
                    {
                        NumberFormatBuilder::rebuild(lexical_core::format::RUST_LITERAL)
                            .mantissa_radix(16)
                            .build()
                    },
                >(delta, &PARSE_INTEGER_OPTS)
                .or_else(|_| self.raise_at(start + 1, ErrorKind::InvalidUnicodeEscape))?;

                if self.consume("}") {
                    if let Some(chr) = char::from_u32(chr) {
                        Ok(chr)
                    } else {
                        self.raise_at(start + 1, ErrorKind::InvalidUnicodeEscape)
                    }
                } else {
                    self.raise(ErrorKind::Expected("`}`"))
                }
            })
        }
    }

    #[inline]
    fn escape_byte(&mut self) -> Option<Result<u8>> {
        if self.consume("\\") {
            self.__escape_byte().or_else(|| Some(self.__escape_common()))
        } else {
            None
        }
    }

    #[inline]
    fn escape_char(&mut self) -> Option<Result<char>> {
        if self.consume("\\") {
            self.__escape_char()
                .or_else(|| Some(self.__escape_common().map(Into::into)))
        } else {
            None
        }
    }
}

//------------------------------------------------------------------------------

/// NOTE: A name starting with an underscore indicates that
/// the parser does not consume any characters during the lookahead.
enum Kind<'de> {
    _Char,
    _Byte,
    _Bytes,
    _StringOrParagraph,
    Bool(bool),
    UnsFloat,
    NegFloat,
    SpecialFloat(f64),
    UnsInt,
    NegInt,
    LongUnsInt,
    LongNegInt,
    Maybe,
    Tuple,
    Seq,
    Map,
    NominalUnnamed,
    NominalStemOnly { name: &'de str },
    NominalFullNamed { name: &'de str, parent: &'de str },
}

enum NominalKind {
    Unit,
    Tuple,
    Record,
}

impl<'de> Parser<'de> {
    #[inline]
    fn lookahead(&mut self) -> Result<Kind<'_>> {
        if self.consume_ws_("?")? {
            return Ok(Kind::Maybe);
        } else if self.consume_ws_("(")? {
            return Ok(Kind::Tuple);
        } else if self.consume_ws_("[")? {
            return Ok(Kind::Seq);
        } else if self.consume_ws_("{")? {
            return Ok(Kind::Map);
        }

        let long_number = 'non_number: {
            let kind = match self.rest_bytes() {
                [b'\'', ..] => Kind::_Char,

                [b'b', b'\'', ..] => Kind::_Byte,

                [b'b', b'"' | b'`', ..]
                | [b'b', b'1', b'6', b'"', ..]
                | [b'b', b'3', b'2', b'"', ..]
                | [b'b', b'6', b'4', b'"', ..] => Kind::_Bytes,

                [b'"', ..] | [b'`', b'`' | b'"' | b'|', ..] => Kind::_StringOrParagraph,

                [b'-' | b'0'..=b'9', ..] => break 'non_number false,

                [_, ..] => match self.consume_keyword_or_ident_or_underscore()? {
                    Token::Keyword(kw) => match kw {
                        Keyword::Long => break 'non_number true,
                        Keyword::True => Kind::Bool(true),
                        Keyword::False => Kind::Bool(false),
                        Keyword::Infinity => Kind::SpecialFloat(f64::NAN),
                        Keyword::NotANumber => Kind::SpecialFloat(f64::INFINITY),
                    },

                    Token::Identifier(name) => {
                        if self.consume_ws_("::")? {
                            let parent = name;
                            let name = self.consume_ident()?;

                            Kind::NominalFullNamed { name, parent }
                        } else {
                            Kind::NominalStemOnly { name }
                        }
                    }

                    Token::Underscore => Kind::NominalUnnamed,
                },

                [] => return self.raise(ErrorKind::ExpectedValue),
            };

            return Ok(kind);
        };

        let kind = if long_number {
            if self.consume_ws_("-")? {
                Kind::LongNegInt
            } else {
                Kind::LongUnsInt
            }
        } else if self.consume_ws_("-")? {
            if let Some(b'.' | b'e' | b'E') = self.rest_bytes().iter().find(|byte| byte.is_ascii_digit()) {
                Kind::NegFloat
            } else {
                Kind::NegInt
            }
        } else if let Some(b'.' | b'e' | b'E') = self.rest_bytes().iter().find(|byte| byte.is_ascii_digit()) {
            Kind::UnsFloat // lexical-core can handle `inf` and `NaN`.
        } else {
            Kind::UnsInt
        };

        Ok(kind)
    }

    #[inline]
    fn lookahead_nominal(&mut self) -> Result<NominalKind> {
        if self.adjacent_to_delim() {
            Ok(NominalKind::Unit)
        } else if self.consume_ws_("(")? {
            Ok(NominalKind::Tuple)
        } else if self.consume_ws_("{")? {
            Ok(NominalKind::Record)
        } else {
            self.raise(ErrorKind::ExpectedNominalValue)
        }
    }
}

//------------------------------------------------------------------------------

enum Either<L, R> {
    Left(L),
    Right(R),
}

macro_rules! maybe_deserialize_baseXX {
    ( $self:ident, $indicator:literal, $decoder:ident ) => {{
        if $self.consume($indicator) {
            let rest = $self.rest_bytes();
            let Some(off) = memchr::memchr(b'"', rest) else {
                return $self.raise_unexpected_end();
            };

            let buf = $decoder.decode(&rest[..off]).map_err(|e| {
                let mut e: Error = e.into();
                e.pos += $self.pos;
                e
            })?;

            $self.bump(off + 1);
            $self.consume_whitespace_comment()?;

            return Ok(Either::Right(buf));
        }
    }};
}

impl<'de> Parser<'de> {
    #[inline]
    fn parse_char(&mut self) -> Result<char> {
        let start = self.pos;

        'char: {
            if !self.consume("'") {
                break 'char;
            }

            let ch = if let Some(ch) = self.escape_char() {
                ch?
            } else if let Some(ch) = self.rest().chars().next() {
                self.bump(ch.len_utf8());
                ch
            } else {
                break 'char;
            };

            if !self.consume_ws_("'")? {
                return self.raise(ErrorKind::Expected("`'`"));
            }

            return Ok(ch);
        }

        self.raise_at(start, ErrorKind::ExpectedCharacter)
    }

    #[inline]
    fn parse_byte(&mut self) -> Result<u8> {
        let start = self.pos;

        'byte: {
            if !self.consume("b'") {
                break 'byte;
            }

            let byte = if let Some(byte) = self.escape_byte() {
                byte?
            } else if let Some(byte) = self.rest().chars().next().filter(|ch| ch.is_ascii()) {
                self.bump(1);
                byte as u8
            } else {
                break 'byte;
            };

            if !self.consume_ws_("'")? {
                return self.raise(ErrorKind::Expected("`'`"));
            }

            return Ok(byte);
        }

        self.raise_at(start, ErrorKind::ExpectedByteInteger)
    }

    #[inline]
    fn parse_byte_string(&mut self) -> Result<Either<&'de [u8], ByteBuf>> {
        #[inline]
        fn filter_non_ascii(s: &str, der: &mut Parser) -> Result<()> {
            match s.as_bytes().iter().enumerate().find(|(_off, byte)| !byte.is_ascii()) {
                Some((off, _byte)) => der.raise_at(der.pos - s.len() + off, ErrorKind::NonAsciiByteString),
                None => Ok(()),
            }
        }

        let start = self.pos;

        'byte_string: {
            if !self.consume("b") {
                break 'byte_string;
            }

            maybe_deserialize_baseXX!(self, "64\"", BASE64URL_NOPAD);
            maybe_deserialize_baseXX!(self, "32\"", BASE32_NOPAD);
            maybe_deserialize_baseXX!(self, "16\"", HEXUPPER_PERMISSIVE);

            let delim_len = self.consume_while_fast(is_backtick).len();

            if !self.consume("\"") {
                break 'byte_string;
            }

            let mut buf = Vec::new();
            let mut cursor = self.pos; // initialized as `inner_start`.
            let mut inner_end;

            match delim_len {
                0 => loop {
                    /* normal byte string */
                    let Some(off) = memchr::memchr3(b'\"', b'\\', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    filter_non_ascii(self.bump(off), self)?;

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            self.consume_newline()?;
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(b'\n');
                        }
                        b'\\' => {
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(self.escape_byte().unwrap()?);
                        }
                        b'\"' => {
                            inner_end = self.pos;
                            self.bump(1);
                            break;
                        }
                        _ => unreachable!(),
                    }

                    cursor = self.pos;
                },

                _ => loop {
                    /* raw byte string */
                    let Some(off) = memchr::memchr2(b'\"', b'\r', self.rest_bytes()) else {
                        return self.raise_unexpected_end();
                    };

                    filter_non_ascii(self.bump(off), self)?;

                    match self.peek_byte().unwrap() {
                        b'\r' => {
                            self.consume_newline()?;
                            buf.extend_from_slice(&self.source_bytes()[cursor..self.pos]);
                            buf.push(b'\n');
                        }
                        b'\"' => {
                            inner_end = self.pos;
                            self.bump(1);
                            match self.consume_while_fast(is_backtick).len().cmp(&delim_len) {
                                Ordering::Less => (),
                                Ordering::Equal => break,
                                Ordering::Greater => {
                                    return self.raise_at(inner_end + 1, ErrorKind::UnbalancedRawDelimiters)
                                }
                            }
                        }
                        _ => unreachable!(),
                    }

                    cursor = self.pos;
                },
            }

            self.consume_whitespace_comment()?;

            return if buf.is_empty() {
                Ok(Either::Left(&self.source_bytes()[cursor..inner_end]))
            } else {
                buf.extend_from_slice(&self.source_bytes()[cursor..inner_end]);
                Ok(Either::Right(buf))
            };
        }

        self.raise_at(start, ErrorKind::ExpectedByteString)
    }

    fn parse_string_or_paragraph(&mut self) -> Result<Either<&'de str, String>> {
        let start = self.pos;
        let delim_len = self.consume_while_fast(is_backtick).len();

        if self.consume("\"") {
            self._parse_string(delim_len)
        } else if delim_len > 0 && self.consume("|") {
            self._parse_paragraph(delim_len)
        } else {
            self.raise_at(start, ErrorKind::ExpectedStringOrParagraph)
        }
    }

    #[inline]
    fn _parse_string(&mut self, delim_len: usize) -> Result<Either<&'de str, String>> {
        let mut buf = String::new();
        let mut cursor = self.pos; // initialized as `inner_start`.
        let mut inner_end;

        match delim_len {
            0 => loop {
                /* normal string */
                let Some(off) = memchr::memchr3(b'\"', b'\\', b'\r', self.rest_bytes()) else {
                    return self.raise_unexpected_end();
                };

                self.bump(off);

                match self.peek_byte().unwrap() {
                    b'\r' => {
                        self.consume_newline()?;
                        buf.push_str(&self.src[cursor..self.pos]);
                        buf.push('\n');
                    }
                    b'\\' => {
                        buf.push_str(&self.src[cursor..self.pos]);
                        buf.push(self.escape_char().unwrap()?);
                    }
                    b'\"' => {
                        inner_end = self.pos;
                        self.bump(1);
                        break;
                    }
                    _ => unreachable!(),
                }

                cursor = self.pos;
            },

            _ => loop {
                /* raw string */
                let Some(off) = memchr::memchr2(b'\"', b'\r', self.rest_bytes()) else {
                    return self.raise_unexpected_end();
                };

                self.bump(off);

                match self.peek_byte().unwrap() {
                    b'\r' => {
                        self.consume_newline()?;
                        buf.push_str(&self.src[cursor..self.pos]);
                        buf.push('\n');
                    }
                    b'\"' => {
                        inner_end = self.pos;
                        self.bump(1);
                        match self.consume_while_fast(is_backtick).len().cmp(&delim_len) {
                            Ordering::Less => (),
                            Ordering::Equal => break,
                            Ordering::Greater => {
                                return self.raise_at(inner_end + 1, ErrorKind::UnbalancedRawDelimiters)
                            }
                        }
                    }
                    _ => unreachable!(),
                }

                cursor = self.pos;
            },
        }

        self.consume_whitespace_comment()?;

        if buf.is_empty() {
            Ok(Either::Left(&self.src[cursor..inner_end]))
        } else {
            buf.push_str(&self.src[cursor..inner_end]);
            Ok(Either::Right(buf))
        }
    }

    #[inline]
    fn _parse_paragraph(&mut self, delim_len: usize) -> Result<Either<&'de str, String>> {
        #[inline]
        fn trim(s: &str) -> &str {
            if let Some((b' ', s)) = s.as_bytes().split_first() {
                unsafe { core::str::from_utf8_unchecked(s) }
            } else {
                s
            }
            .trim_end()
        }

        let mut buf = String::new();
        let mut firstline;
        match memchr::memchr2(b'\r', b'\n', self.rest_bytes()) {
            None => firstline = Some(trim(self.bump_to_end())),
            Some(off) => {
                firstline = Some(trim(self.bump(off)));
                self.consume_newline()?;
                self.consume_whitespace_comment()?;
            }
        }

        loop {
            if self.adjacent_to_delim() {
                break;
            }
            if self.consume_while_fast(is_backtick).len() != delim_len {
                return self.raise(ErrorKind::UnbalancedRawDelimiters);
            }
            let Some(sym) = self.peek_byte() else {
                return self.raise(ErrorKind::InvalidParagraphLine);
            };
            if !matches!(sym, b'|' | b'<' | b'>') {
                return self.raise(ErrorKind::InvalidParagraphLine);
            }

            self.bump(1);

            let contiline;
            match memchr::memchr2(b'\r', b'\n', self.rest_bytes()) {
                None => contiline = trim(self.bump_to_end()),
                Some(off) => {
                    contiline = trim(self.bump(off));
                    self.consume_newline()?;
                    self.consume_whitespace_comment()?;
                }
            }

            if let Some(first) = firstline.take() {
                buf.push_str(first);
            }
            if sym == b'|' {
                buf.push('\n');
            }
            if sym == b'>' && !contiline.is_empty() {
                buf.push(' ');
            }

            buf.push_str(contiline);
        }

        if let Some(s) = firstline {
            Ok(Either::Left(s))
        } else {
            Ok(Either::Right(buf))
        }
    }
}
