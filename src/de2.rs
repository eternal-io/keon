use super::*;
use crate::error::*;
use core::num::NonZeroU8;
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use kaparser::*;
use lexical_core::{
    parse_partial_with_options, parse_with_options, NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder,
    ParseIntegerOptions, ParseIntegerOptionsBuilder,
};
use serde::de::{
    value::{EnumAccessDeserializer, StrDeserializer},
    DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use smol_str::SmolStr;

const NUMBER_FMT: u128 = NumberFormatBuilder::rebuild(lexical_core::format::RUST_STRING)
    .no_special(false)
    .case_sensitive_exponent(false)
    .case_sensitive_special(true)
    .case_sensitive_base_prefix(true)
    .build();
const NUMBER_FMT_BIN: u128 = NumberFormatBuilder::rebuild(NUMBER_FMT)
    .mantissa_radix(2)
    .base_prefix(NonZeroU8::new(b'b'))
    .build();
const NUMBER_FMT_OCT: u128 = NumberFormatBuilder::rebuild(NUMBER_FMT)
    .mantissa_radix(8)
    .base_prefix(NonZeroU8::new(b'o'))
    .build();
const NUMBER_FMT_HEX: u128 = NumberFormatBuilder::rebuild(NUMBER_FMT)
    .mantissa_radix(16)
    .base_prefix(NonZeroU8::new(b'x'))
    .build();

const PARSE_OPTS_INT: &ParseIntegerOptions = &ParseIntegerOptionsBuilder::new()
    .no_multi_digit(false)
    .build_unchecked();
const PARSE_OPTS_FLOAT: &ParseFloatOptions = &ParseFloatOptionsBuilder::new()
    .lossy(false)
    .exponent(b'e')
    .decimal_point(b'.')
    .nan_string(Some(b"NaN"))
    .inf_string(Some(b"inf"))
    .infinity_string(None)
    .build_unchecked();

kaparser::token_set! {
    Lookahead {
        Parenthesis = "(",
        Bracket     = "[",
        Brace       = "{",

        String      = "\"",
        StringRaw1  = "`\"",
        StringRaw2  = "``",

        BytesNormal = "b\"",
        BytesRaw    = "b`",
        BytesBase64 = "b64\"",
        BytesBase32 = "b32\"",
        BytesBase16 = "b16\"",

        RawIdent    = "`",
        Option      = "?",
        Mayary      = "%",
        Character   = "'",
        Paragraph   = "|",
        Comment     = "/",

        BoolTrue    = "true",
        BoolFalse   = "false",
        FloatNaN    = "NaN",
        FloatInf    = "inf",
        FloatNegInf = "-inf",
    }

    BlockComment {
        BlockEnter  = "/*",
        BlockLeave  = "*/",
    }

    KeyToValue {
        Colon       = ":",
        FatArrow    = "=>",
    }

    Delimiter {
        FatArrow    = "=>",
        Parenthesis = ")",
        Bracket     = "]",
        Brace       = "}",
        Comma       = ",",
        Semi        = ";",
    }
}

enum BaseXX {
    Base16,
    Base32,
    Base64,
}

//==================================================================================================

pub struct Deserializer<'de, R: Read> {
    par: Utf8Parser<'de, R>,
    ttl: usize,
}

impl<'de> Deserializer<'de, Slice> {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(slice: &'de str) -> Self {
        Self::new(Utf8Parser::from_str(slice))
    }

    pub fn from_bytes(bytes: &'de [u8]) -> Result<Self> {
        Utf8Parser::from_bytes(bytes).map(Self::new).map_err(Error::from)
    }
}

impl<'de, R: Read> Deserializer<'de, R> {
    pub fn from_reader(reader: R) -> Self {
        Self::new(Utf8Parser::from_reader(reader))
    }

    pub fn from_parser(parser: Utf8Parser<'de, R>) -> Self {
        Self::new(parser)
    }

    pub fn into_parser(self) -> Utf8Parser<'de, R> {
        self.par
    }

    #[inline(always)]
    fn new(par: Utf8Parser<'de, R>) -> Self {
        Self {
            par,
            ttl: RECURSION_LIMIT,
        }
    }

    //------------------------------------------------------------------------------

    fn situate(&self, situation: &mut Error) {
        self.par.situate(situation);
    }

    //------------------------------------------------------------------------------

    fn skip_comment(&mut self) -> Result<()> {
        match self.par.next()? {
            None => return Error::raise(ErrorKind::UnexpectedEof),
            Some(ch) => match ch {
                '/' => {
                    self.par.skip_till(is_newline)?;
                }

                '*' => {
                    let mut depth = 1;
                    while depth != 0 {
                        match self.par.skip_until(BlockCommentTokens)?.1 {
                            None => return Error::raise(ErrorKind::UnexpectedEof),
                            Some(t) => match t {
                                BlockCommentToken::BlockEnter => depth += 1,
                                BlockCommentToken::BlockLeave => depth -= 1,
                            },
                        }
                    }
                }

                _ => return Error::raise(ErrorKind::UnexpectedToken),
            },
        }

        Ok(())
    }

    /// 🔒 Will use selection.
    fn scan_ident(&mut self) -> Result<SmolStr> {
        self.par.begin_select();

        if self.par.take_once(('_', is_xid_start))?.is_none() {
            return Error::raise_working(ErrorKind::UnexpectedToken, OriginallyWant::Identifier);
        }

        self.par.take_while(is_xid_continue)?;

        Ok(SmolStr::from(self.par.commit_select().unwrap()))
    }

    /// Requires the leading backslash `\` has been consumed.
    /// Hex escape from `\x00` to at most `\xFF`.
    fn scan_escape_byte(&mut self) -> Result<u8> {
        let comm = ErrorKind::InvalidBytesEscape;
        let want = OriginallyWant::LiteralBytes;
        let Some(ch) = self.par.next()? else {
            return Error::raise_working(ErrorKind::UnexpectedEof, want);
        };

        Ok(match ch {
            '\\' => b'\\',
            '\"' => b'\"',
            '\'' => b'\'',
            'n' => b'\n',
            't' => b'\t',
            'x' => lexical_core::parse(
                self.par
                    .take_times(is_hexdigit, 2)?
                    .ok_or(())
                    .or(Error::raise_working(comm, want))?
                    .0
                    .as_bytes(),
            )
            .unwrap(),

            _ => return Error::raise_working(comm, want),
        })
    }

    /// Requires the leading backslash `\` has been consumed.
    /// Hex escape from `\x00` to at most `\x7F`.
    fn scan_escape_char(&mut self) -> Result<char> {
        let want = OriginallyWant::LiteralString;
        let Some(ch) = self.par.next()? else {
            return Error::raise_working(ErrorKind::UnexpectedEof, want);
        };

        Ok(match ch {
            '\\' => '\\',
            '\"' => '\"',
            '\'' => '\'',
            'n' => '\n',
            't' => '\t',
            'x' => {
                if let Some((s, _)) = self.par.take_times(is_hexdigit, 2)? {
                    if let n @ 0x00..=0x7F = lexical_core::parse::<u8>(s.as_bytes()).unwrap() {
                        return Ok(char::from(n));
                    }
                }

                return Error::raise_working(ErrorKind::InvalidAsciiEscape, want);
            }
            'u' => {
                if self.par.take_once('{')?.is_some() {
                    let (s, p) = &self.par.take_while(('_', is_hexdigit))?;
                    if let Some('}') = p {
                        if let Ok(n) = parse_with_options::<u32, NUMBER_FMT_HEX>(s.as_bytes(), PARSE_OPTS_INT) {
                            if let Ok(ch) = char::try_from(n) {
                                return Ok(ch);
                            }
                        }
                    }
                }

                return Error::raise_working(ErrorKind::InvalidUnicodeEscape, want);
            }

            _ => return Error::raise_working(ErrorKind::InvalidStringEscape, want),
        })
    }

    //------------------------------------------------------------------------------

    fn parse<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        let (ttl, overflowed) = self.ttl.overflowing_sub(1);
        if overflowed {
            return Error::raise(ErrorKind::ExceededRecursionLimit);
        }

        self.ttl = ttl;

        let val = loop {
            let Some(ch) = self.par.take_while(is_whitespace)?.1 else {
                break Error::raise(ErrorKind::UnexpectedEof);
            };

            break match self.par.matches(LookaheadTokens)? {
                Some(t) => match t {
                    LookaheadToken::Parenthesis => self.parse_parenthesis(vis),
                    LookaheadToken::Bracket => self.parse_seq(vis),
                    LookaheadToken::Brace => self.parse_map(vis),

                    LookaheadToken::String => self.parse_string(vis, 0),
                    LookaheadToken::StringRaw1 => self.parse_string(vis, 1),
                    LookaheadToken::StringRaw2 => self.parse_string(vis, 2),

                    LookaheadToken::BytesNormal => self.parse_bytes(vis, 0),
                    LookaheadToken::BytesRaw => self.parse_bytes(vis, 1),
                    LookaheadToken::BytesBase64 => self.parse_bytes_encoding(vis, BaseXX::Base64),
                    LookaheadToken::BytesBase32 => self.parse_bytes_encoding(vis, BaseXX::Base32),
                    LookaheadToken::BytesBase16 => self.parse_bytes_encoding(vis, BaseXX::Base16),

                    LookaheadToken::RawIdent => {
                        self.par.bump(1);
                        self.parse_enum(vis)
                    }

                    LookaheadToken::Option => self.parse_option(vis),
                    LookaheadToken::Mayary => self.parse_mayary(vis),
                    LookaheadToken::Character => self.parse_character(vis),
                    LookaheadToken::Paragraph => self.parse_paragraph(vis),

                    LookaheadToken::Comment => {
                        self.skip_comment()?;
                        continue;
                    }

                    LookaheadToken::BoolTrue => vis.visit_bool(true),
                    LookaheadToken::BoolFalse => vis.visit_bool(false),
                    LookaheadToken::FloatNaN => vis.visit_f32(f32::NAN),
                    LookaheadToken::FloatInf => vis.visit_f32(f32::INFINITY),
                    LookaheadToken::FloatNegInf => vis.visit_f32(f32::NEG_INFINITY),
                },

                None => match ('-', is_digit).predicate(ch) {
                    true => self.parse_number(vis, ch),
                    false => self.parse_enum(vis),
                },
            };
        };

        self.ttl += 1;

        val
    }

    fn parse_number<V: Visitor<'de>>(&mut self, vis: V, peeked: char) -> Result<V::Value> {
        self.par.begin_select();

        let neg = if peeked == '-' {
            self.par.bump(1);
            true
        } else {
            false
        };

        if peeked == '0' {
            self.par.bump(1);

            if let Some(t) = self.par.take_once(('x', 'o', 'b'))? {
                self.par.commit_select(); // TODO: 把基数前缀全部取消掉！！！
                self.par.pull_at_least(100)?;

                let input = self.par.content().as_bytes();
                let (v, len) = match neg {
                    true => {
                        let (v, len) = match t {
                            'x' => parse_partial_with_options::<_, NUMBER_FMT_HEX>(input, PARSE_OPTS_INT),
                            'o' => parse_partial_with_options::<_, NUMBER_FMT_OCT>(input, PARSE_OPTS_INT),
                            'b' => parse_partial_with_options::<_, NUMBER_FMT_BIN>(input, PARSE_OPTS_INT),
                            _ => unreachable!(),
                        }
                        .map_err(|e| Error::from(e).want(OriginallyWant::LiteralSignedInteger))?;

                        (vis.visit_i64(v), len)
                    }
                    false => {
                        let (v, len) = match t {
                            'x' => parse_partial_with_options::<_, NUMBER_FMT_HEX>(input, PARSE_OPTS_INT),
                            'o' => parse_partial_with_options::<_, NUMBER_FMT_OCT>(input, PARSE_OPTS_INT),
                            'b' => parse_partial_with_options::<_, NUMBER_FMT_BIN>(input, PARSE_OPTS_INT),
                            _ => unreachable!(),
                        }
                        .map_err(|e| Error::from(e).want(OriginallyWant::LiteralUnsignedInteger))?;

                        (vis.visit_u64(v), len)
                    }
                };

                self.par.bump(len);

                return v;
            }
        }

        if let Some(ch) = self.par.take_while(('_', is_digit))?.1 {
            if ('.', 'e', 'E').predicate(ch) {
                self.par.rollback_select();
                self.par.pull_at_least(100)?;

                let input = self.par.content().as_bytes();
                let (v, len) = parse_partial_with_options::<_, NUMBER_FMT>(input, PARSE_OPTS_FLOAT)
                    .map_err(|e| Error::from(e).want(OriginallyWant::LiteralFloatNumber))?;

                self.par.bump(len);

                return vis.visit_f64(v);
            }
        }

        let input = self.par.commit_select().unwrap().as_bytes();
        let v = parse_with_options::<_, NUMBER_FMT>(input, PARSE_OPTS_INT)
            .map_err(|e| Error::from(e).want(OriginallyWant::LiteralUnsignedInteger))?;

        vis.visit_u64(v)
    }

    fn parse_character<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        self.par.begin_select();

        let want = OriginallyWant::LiteralCharacter;
        let Some(mut ch) = self.par.next()? else {
            return Error::raise_working(ErrorKind::UnexpectedEof, want);
        };

        match ch {
            '\\' => ch = self.scan_escape_char()?,
            '\n' => return Error::raise_working(ErrorKind::UnexpectedNewline, want),
            '\'' => return Error::raise_working(ErrorKind::InvalidCharacterTooLess, want),
            _ => (),
        }

        if self.par.take_once('\'')?.is_none() {
            return Error::raise_working(ErrorKind::InvalidCharacterTooMany, want);
        }

        self.par.commit_select();

        vis.visit_char(ch)
    }

    fn parse_string<V: Visitor<'de>>(&mut self, vis: V, mut n_backtick: usize) -> Result<V::Value> {
        if n_backtick == 2 {
            n_backtick += self.par.take_while('`')?.0.len();
        }

        match n_backtick {
            0 => {
                self.par.begin_select();

                let mut buf: Option<String> = None;
                loop {
                    if self.par.exhausted() {
                        return Error::raise_working(ErrorKind::UnexpectedEof, OriginallyWant::LiteralString);
                    }
                    if let Some((s, p)) = self.par.skim_till(('"', '\\'))? {
                        match p {
                            '"' => {
                                if let Some(buf) = buf.as_mut() {
                                    buf.push_str(s);
                                }
                                break;
                            }
                            '\\' => {
                                let buf = buf.get_or_insert_default();
                                buf.push_str(s);
                                buf.push(self.scan_escape_char()?);
                            }
                            _ => unreachable!(),
                        }
                    }
                }

                let s = self.par.commit_select().unwrap();
                match buf {
                    None => vis.visit_str(s),
                    Some(s) => vis.visit_string(s),
                }
            }

            n => {
                self.par.begin_select();

                loop {
                    if self.par.exhausted() || self.par.skim_till('"')?.is_none() {
                        return Error::raise_working(ErrorKind::UnexpectedEof, OriginallyWant::LiteralStringRaw);
                    }
                    if self.par.take_times('`', n)?.is_some() {
                        break;
                    }
                }

                vis.visit_str(self.par.commit_select().unwrap())
            }
        }
    }

    fn parse_bytes<V: Visitor<'de>>(&mut self, vis: V, mut n_backtick: usize) -> Result<V::Value> {
        if n_backtick == 1 {
            n_backtick += self.par.take_while('`')?.0.len();
        }

        match n_backtick {
            0 => {
                self.par.begin_select();

                let mut buf: Option<Vec<u8>> = None;
                loop {
                    if self.par.exhausted() {
                        return Error::raise_working(ErrorKind::UnexpectedEof, OriginallyWant::LiteralBytes);
                    }
                    if let Some((s, p)) = self.par.skim_till(('"', '\\'))? {
                        match p {
                            '"' => {
                                if let Some(buf) = buf.as_mut() {
                                    buf.extend_from_slice(s.as_bytes());
                                }
                                break;
                            }
                            '\\' => {
                                let buf = buf.get_or_insert_default();
                                buf.extend_from_slice(s.as_bytes());
                                buf.push(self.scan_escape_byte()?);
                            }
                            _ => unreachable!(),
                        }
                    }
                }

                let bytes = self.par.commit_select().unwrap().as_bytes();
                match buf {
                    None => vis.visit_bytes(bytes),
                    Some(buf) => vis.visit_byte_buf(buf),
                }
            }

            n => {
                self.par.begin_select();

                loop {
                    if self.par.exhausted() || self.par.skim_till('"')?.is_none() {
                        return Error::raise_working(ErrorKind::UnexpectedEof, OriginallyWant::LiteralBytesRaw);
                    }
                    if self.par.take_times('`', n)?.is_some() {
                        break;
                    }
                }

                vis.visit_bytes(self.par.commit_select().unwrap().as_bytes())
            }
        }
    }

    fn parse_bytes_encoding<V: Visitor<'de>>(&mut self, vis: V, flavor: BaseXX) -> Result<V::Value> {
        self.par.begin_select();

        let Some((s, _)) = self.par.skim_till('"')? else {
            return Error::raise_working(ErrorKind::UnexpectedEof, OriginallyWant::LiteralBytesEncoding);
        };
        let input = s.as_bytes();
        let buf = match flavor {
            BaseXX::Base16 => HEXUPPER_PERMISSIVE.decode(input),
            BaseXX::Base32 => BASE32_NOPAD.decode(input),
            BaseXX::Base64 => BASE64URL_NOPAD.decode(input),
        }
        .map_err(|e| Error::from(e).want(OriginallyWant::LiteralBytesEncoding))?;

        self.par.commit_select();

        vis.visit_byte_buf(buf)
    }

    fn parse_mayary<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn parse_option<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn parse_parenthesis<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn parse_tuple<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn parse_seq<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        todo!()
    }

    fn parse_map<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        todo!()
    }

    /// - Nameness: `Difficulty::Easy`.
    /// - Nameless: `Medium`, `Hard { heart: 1 }`.
    fn parse_enum<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        let mut name = self.scan_ident()?;
        self.par.take_while(is_whitespace)?;

        if self.par.matches("::")?.is_some() {
            name = self.scan_ident()?;
            self.par.take_while(is_whitespace)?;
        }

        vis.visit_enum(EnumAccessor::new(self, name))
    }

    fn parse_paragraph<V: Visitor<'de>>(&mut self, vis: V) -> Result<V::Value> {
        fn trim(mut s: &str) -> &str {
            s = s.strip_prefix("\x20").unwrap_or(s);
            s.as_bytes()
                .iter()
                .cloned()
                .map(char::from)
                .rposition(not!(is_whitespace_nn))
                .map(|n| &s[..=n])
                .unwrap_or(s)
        }

        self.par.begin_select();

        let first_len = self.par.skip_till('\n')?.0.len();
        let mut buf: Option<String> = None;
        let mut newlined = false;

        loop {
            match self.par.take_while(is_whitespace_nn)?.1 {
                None => break,
                Some(ch) => {
                    if !('|', '<', '`').predicate(ch) {
                        break;
                    }

                    let s = match buf.as_mut() {
                        Some(buf) => buf,
                        None => {
                            buf = Some(String::from(trim(&self.par.commit_select().unwrap()[..first_len])));
                            buf.as_mut().unwrap()
                        }
                    };
                    let line = trim(self.par.skip_till('\n')?.0);

                    if ch == '`' {
                        newlined = line.is_empty();
                        s.push_str("\n");
                        s.push_str(line);
                    } else {
                        match line.is_empty() {
                            true => {
                                if !newlined {
                                    newlined = true;
                                    s.push_str("\n");
                                }
                            }
                            false => {
                                if !newlined && ch == '|' {
                                    s.push_str("\x20");
                                }
                                newlined = false;
                                s.push_str(line);
                            }
                        }
                    }
                }
            }
        }

        match buf {
            None => vis.visit_str(trim(&self.par.commit_select().unwrap()[..first_len])),
            Some(s) => vis.visit_string(s),
        }
    }
}

impl<'de, R: Read> serde::Deserializer<'de> for &mut Deserializer<'de, R> {
    type Error = Error;

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V: Visitor<'de>>(self, vis: V) -> Result<V::Value> {
        self.parse(vis).map_err(|mut e| {
            self.situate(&mut e);
            e
        })
    }
}

//==================================================================================================

struct EnumAccessor<'z, 'de, R: Read> {
    der: &'z mut Deserializer<'de, R>,
    variant: SmolStr,
}

impl<'z, 'de, R: Read> EnumAccessor<'z, 'de, R> {
    /// Requires the leading `Enum::Variant` has been consumed, and the `Variant` must be provided in parameter.
    fn new(der: &'z mut Deserializer<'de, R>, variant: SmolStr) -> Self {
        Self { der, variant }
    }
}

impl<'z, 'de, R: Read> EnumAccess<'de> for EnumAccessor<'z, 'de, R> {
    type Error = Error;
    type Variant = VariantAccessor<'z, 'de, R>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        Ok((
            seed.deserialize(StrDeserializer::<Error>::new(&self.variant))?,
            VariantAccessor::new(self.der),
        ))
    }
}

struct VariantAccessor<'z, 'de, R: Read> {
    der: &'z mut Deserializer<'de, R>,
}

impl<'z, 'de, R: Read> VariantAccessor<'z, 'de, R> {
    fn new(der: &'z mut Deserializer<'de, R>) -> Self {
        Self { der }
    }
}

impl<'de, R: Read> VariantAccess<'de> for VariantAccessor<'_, 'de, R> {
    type Error = Error;

    /// Note that inputs like `Variant()` is a nullary tuple variant instead.
    fn unit_variant(self) -> Result<()> {
        if self.der.par.matches(DelimiterTokens)?.is_none() {
            return Error::raise(ErrorKind::ExpectedUnitVariant);
        }

        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        match self.der.par.take_once(('%', '('))? {
            None => Error::raise(ErrorKind::ExpectedNewtypeVariant),
            Some(ch) => match ch {
                '%' => seed.deserialize(&mut *self.der),
                '(' => {
                    let val = seed.deserialize(&mut *self.der)?;

                    self.der.par.take_once(',')?;
                    if self.der.par.take_once(')')?.is_none() {
                        return Error::raise(ErrorKind::ExpectedNewtypeVariant);
                    }

                    Ok(val)
                }
                _ => unreachable!(),
            },
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _: usize, vis: V) -> Result<V::Value> {
        match self.der.par.take_once(('%', '('))? {
            None => Error::raise(ErrorKind::ExpectedTupleVariant),
            Some(ch) => match ch {
                '%' => todo!(), // parse_nullary(vis)
                '(' => todo!(), // parse_tuple::<_, true>(self.der, vis),
                _ => unreachable!(),
            },
        }
    }

    fn struct_variant<V: Visitor<'de>>(self, _: &'static [&'static str], vis: V) -> Result<V::Value> {
        if self.der.par.take_once('{')?.is_none() {
            return Error::raise(ErrorKind::ExpectedStructVariant);
        }

        self.der.parse_map(vis)
    }
}
