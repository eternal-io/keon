//! Everything has to fail early... mainly because this crate is not designed for editors,
//! even if [logos] allows us to gather all errors.

use super::{value::*, *};
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use lexical_core::{
    parse_with_options, NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions,
    ParseIntegerOptionsBuilder,
};
use logos::{FilterResult, Lexer, Logos, Skip};
use std::{cell::RefCell, cmp::Ordering, num::NonZeroU8, rc::Rc};

pub(crate) type LexerResult<T> = core::result::Result<T, ErrorKind>;

//==================================================================================================

const NUMBER_FMT: u128 = NumberFormatBuilder::rebuild(lexical_core::format::RUST_STRING)
    .no_special(false)
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

const fn raise(kind: ErrorKind) -> LexerResult<()> {
    Err(kind)
}

//==================================================================================================

#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Ident, Literal,
    Comma, Question,
    Colon, PathSep,
    Percent, FatArrow,
    Paren_, _Paren,
    Brack_, _Brack,
    Brace_, _Brace,
}

impl TokenKind {
    /// `)`, `]`, `}`, `,`, `:` and `=>`.
    pub(crate) fn is_delimiter(&self) -> bool {
        matches!(
            self,
            TokenKind::Comma
                | TokenKind::Colon
                | TokenKind::FatArrow
                | TokenKind::_Paren
                | TokenKind::_Brack
                | TokenKind::_Brace
        )
    }
}

impl<'i> Token<'i> {
    pub(crate) fn kind(&self) -> TokenKind {
        match self {
            Token::__ => unreachable!(),

            Token::Ident(_) => TokenKind::Ident,
            Token::Literal(_) => TokenKind::Literal,

            Token::Comma => TokenKind::Comma,
            Token::Colon => TokenKind::Colon,
            Token::Percent => TokenKind::Percent,
            Token::PathSep => TokenKind::PathSep,
            Token::Question => TokenKind::Question,
            Token::FatArrow => TokenKind::FatArrow,

            Token::Paren_ => TokenKind::Paren_,
            Token::_Paren => TokenKind::_Paren,
            Token::Brack_ => TokenKind::Brack_,
            Token::_Brack => TokenKind::_Brack,
            Token::Brace_ => TokenKind::Brace_,
            Token::_Brace => TokenKind::_Brace,
        }
    }
}

//==================================================================================================

#[rustfmt::skip]
#[derive(Debug, Logos, Clone, PartialEq, PartialOrd)]
#[logos(error = ErrorKind, extras = Extras)]
pub(crate) enum Token<'src> {
    #[token( "//", cb::line_comment)]
    #[token( "/*", cb::block_comment)]
    #[regex(r"\n", |lex| { cb::newline(lex); Skip })]
    #[regex(r"[\t\r\f\v ]+", |_| Skip)] __, // uninhabited.

    #[regex( r".", callback = |lex| cb::ident(lex, &lex.source()[lex.span().start..]),     priority = 0)]
    #[regex(r"`.", callback = |lex| cb::ident(lex, &lex.source()[lex.span().start + 1..]), priority = 1)]
    Ident(&'src str),

    #[regex(r"(true|false)", cb::bool)]
    #[regex(    r"-?([0-9]_*)+",       |lex| cb::integral(lex, Radix::Dec))]
    #[regex(r"-?0b_*([0-1]_*)+",       |lex| cb::integral(lex, Radix::Bin))]
    #[regex(r"-?0o_*([0-7]_*)+",       |lex| cb::integral(lex, Radix::Oct))]
    #[regex(r"-?0x_*([0-9A-Fa-f]_*)+", |lex| cb::integral(lex, Radix::Hex))]
    // dec     =     r"([0-9]_*)+"
    // dec_alt =   r"_*([0-9]_*)+"  # Allows start with underscore.
    // float   =  fr"-?({dec}((\.{dec})?[Ee][+-]?{dec_alt}|\.({dec})?)|inf|NaN)"
    #[regex(r"-?(([0-9]_*)+((\.([0-9]_*)+)?[Ee][+-]?_*([0-9]_*)+|\.(([0-9]_*)+)?)|inf|NaN)", cb::floating)]
    #[regex(   "\'",       cb::char)]
    #[regex(   "\"",       cb::string)]
    #[regex( "`+\"", |lex| cb::raw_string(lex, lex.slice().len() - 1))]
    #[regex(  "b\"",       cb::bytes)]
    #[regex("b`+\"", |lex| cb::raw_bytes(lex, lex.slice().len() - 2))]
    #[regex("b16\"", |lex| cb::bytes_encoding(lex, BaseXX::Base16))]
    #[regex("b32\"", |lex| cb::bytes_encoding(lex, BaseXX::Base32))]
    #[regex("b64\"", |lex| cb::bytes_encoding(lex, BaseXX::Base64))]
    #[regex(  r"\|[^\n]*", cb::paragraph)]
    Literal(Literal<'src>),

    #[token(",")] Comma,
    #[token(":")] Colon,
    #[token("%")] Percent,
    #[token("?")] Question,
    #[token("::")] PathSep,
    #[token("=>")] FatArrow,

    #[token("(")] Paren_,
    #[token(")")] _Paren,

    #[token("[")] Brack_,
    #[token("]")] _Brack,

    #[token("{")] Brace_,
    #[token("}")] _Brace,
}

#[rustfmt::skip]
#[derive(Debug, Logos)]
#[logos(error = ErrorKind, extras = Extras)]
enum TokenComment {
    #[token("/*")] Block_,
    #[token("*/")] _Block,

    #[token("\n", |lex| { cb::newline(lex); Skip })]
    #[regex(".",  | _ |                     Skip  )] __ // uninhabited.
}

#[rustfmt::skip]
#[derive(Debug, Logos)]
#[logos(error = ErrorKind, extras = Extras)]
pub(crate) enum TokenEscape {
    #[token("\n", cb::newline)] Newline,

    #[token("'")] Prime,
    #[regex("\"`*", |lex| lex.slice().len() - 1)] Quote(usize),

    #[regex(r#"[^'"\\\n]+"#,           priority = 0)] NoEscapeUtf8,
    #[regex(r#"[\x00-\x7F--'"\\\n]+"#, priority = 1)] NoEscapeAscii,

    #[regex(r#"\\."#, callback = |_| raise(ErrorKind::InvalidEscape), priority = 2)]
    #[regex(r#"\\x[0-9A-Fa-f]{2}"#,                                   priority = 3)] EscapeByte,
    #[regex(r#"\\x[0-7][0-9A-Fa-f]|\\["'\\ntr0]"#,                    priority = 4)] EscapeAscii,
    #[regex(r#"\\u\{([0-9A-Fa-f]_*)+\}"#,                             priority = 5)] EscapeUnicode,
}

#[rustfmt::skip]
#[derive(Debug, Logos)]
#[logos(error = ErrorKind, extras = Extras)]
enum TokenNoEscape {
    #[regex("\"`*", |lex| lex.slice().len() - 1)] Quote(usize),
    #[regex(r#"[^"]+"#,            priority = 0)] NoEscapeUtf8,
    #[regex(r#"[\x00-\x7F--"]+"#,  priority = 1)] NoEscapeAscii,

    #[token("\n", |lex| { cb::newline(lex); Skip })] __ // uninhabited.
}

#[rustfmt::skip]
#[derive(Debug, Logos)]
#[logos(error = ErrorKind, extras = Extras)]
enum TokenParagraph {
    #[regex(r"\n[\t\r\v\f ]*\|[^\n]*")] SpaceJoinedLine,
    #[regex(r"\n[\t\r\v\f ]*<[^\n]*")]       JoinedLine,
    #[regex(r"\n[\t\r\v\f ]*`[^\n]*")]      AsIsNewLine,
    #[regex(r"\n[\t\r\v\f ]*(,|:|=>|\)|\]|\})?")] Leave,
}

type Extras = Rc<RefCell<InnerExtras>>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InnerExtras {
    pub(crate) line: u32,
    pub(crate) line_start: usize,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub(crate) enum Literal<'i> {
    Bool(bool),
    Int(i64),
    UInt(u64),
    Float(f64),
    Char(char),
    Str(&'i str),
    String(String),
    Bytes(&'i [u8]),
    ByteBuf(ByteBuf),
}

#[derive(Debug)] #[rustfmt::skip]
pub(crate) enum Radix { Dec, Bin, Oct, Hex }

#[derive(Debug)] #[rustfmt::skip]
pub(crate) enum BaseXX { Base16, Base32, Base64 }

fn switch<'i, Token1, Token2>(lex: &Lexer<'i, Token1>) -> Lexer<'i, Token2>
where
    Token1: Logos<'i, Extras = Extras, Source = str>,
    Token2: Logos<'i, Extras = Extras, Source = str>,
{
    let mut tk = Token2::lexer_with_extras(lex.source(), lex.extras.clone());
    tk.bump(lex.span().end);
    tk
}

mod cb {
    use super::*;

    pub(crate) fn newline<'i, T>(lex: &mut Lexer<'i, T>)
    where
        T: Logos<'i, Extras = Extras>,
    {
        let mut extras = lex.extras.borrow_mut();
        extras.line += 1;
        extras.line_start = lex.span().end;
    }

    pub(crate) fn line_comment<'i>(lex: &mut Lexer<'i, Token<'i>>) -> FilterResult<(), ErrorKind> {
        let j = lex.remainder();
        match j.find('\n') {
            Some(n) => lex.bump(n),
            None => lex.bump(j.len()),
        }

        FilterResult::Skip
    }

    pub(crate) fn block_comment<'i>(lex: &mut Lexer<'i, Token<'i>>) -> FilterResult<(), ErrorKind> {
        let mut tk = switch::<_, TokenComment>(lex);
        let mut ctr = 1;
        let mut cursor = lex.span().end;

        while let Some(t) = match tk.next().transpose() {
            Ok(opt) => opt,
            Err(e) => return FilterResult::Error(e),
        } {
            lex.bump(tk.span().end - cursor);
            cursor = tk.span().end;

            match t {
                TokenComment::Block_ => ctr += 1,
                TokenComment::_Block => ctr -= 1,
                TokenComment::__ => unreachable!(),
            }

            if ctr == 0 {
                return FilterResult::Skip;
            }
        }

        FilterResult::Error(ErrorKind::UnexpectedEof)
    }

    pub(crate) fn ident<'i>(lex: &mut Lexer<'i, Token<'i>>, slice: &'i str) -> LexerResult<&'i str> {
        let mut chs = slice.chars();
        if let Some(start) = chs.next() {
            if unicode_ident::is_xid_start(start) || start == '_' {
                let len = chs
                    .take_while(|ch| unicode_ident::is_xid_continue(*ch))
                    .map(char::len_utf8)
                    .sum();

                lex.bump(len);
                return Ok(&slice[..start.len_utf8() + len]);
            }
        }

        Err(ErrorKind::UnexpectedToken)
    }

    pub(crate) fn bool<'i>(lex: &Lexer<'i, Token<'i>>) -> LexerResult<Literal<'i>> {
        Ok(Literal::Bool(match lex.slice().as_bytes()[0] {
            b't' => true,
            b'f' => false,
            _ => unreachable!(),
        }))
    }

    pub(crate) fn integral<'i>(lex: &Lexer<'i, Token<'i>>, rdx: Radix) -> LexerResult<Literal<'i>> {
        let i = lex.slice().as_bytes();
        let map_err = |e| ErrorKind::InvalidNumber(e);
        Ok(match i[0] == b'-' {
            true => Literal::Int(match rdx {
                Radix::Dec => parse_with_options::<_, NUMBER_FMT>(i, PARSE_OPTS_INT).map_err(map_err)?,
                Radix::Bin => parse_with_options::<_, NUMBER_FMT_BIN>(i, PARSE_OPTS_INT).map_err(map_err)?,
                Radix::Oct => parse_with_options::<_, NUMBER_FMT_OCT>(i, PARSE_OPTS_INT).map_err(map_err)?,
                Radix::Hex => parse_with_options::<_, NUMBER_FMT_HEX>(i, PARSE_OPTS_INT).map_err(map_err)?,
            }),
            false => Literal::UInt(match rdx {
                Radix::Dec => parse_with_options::<_, NUMBER_FMT>(i, PARSE_OPTS_INT).map_err(map_err)?,
                Radix::Bin => parse_with_options::<_, NUMBER_FMT_BIN>(i, PARSE_OPTS_INT).map_err(map_err)?,
                Radix::Oct => parse_with_options::<_, NUMBER_FMT_OCT>(i, PARSE_OPTS_INT).map_err(map_err)?,
                Radix::Hex => parse_with_options::<_, NUMBER_FMT_HEX>(i, PARSE_OPTS_INT).map_err(map_err)?,
            }),
        })
    }

    pub(crate) fn floating<'i>(lex: &Lexer<'i, Token<'i>>) -> LexerResult<Literal<'i>> {
        Ok(Literal::Float(
            parse_with_options::<_, NUMBER_FMT>(lex.slice().as_bytes(), PARSE_OPTS_FLOAT)
                .map_err(ErrorKind::InvalidNumber)?,
        ))
    }

    pub(crate) fn char<'i>(lex: &mut Lexer<'i, Token<'i>>) -> LexerResult<Literal<'i>> {
        let mut tk = switch::<_, TokenEscape>(lex);

        if let Some(t) = tk.next().transpose()? {
            lex.bump(tk.slice().len());
            let ch = match t {
                TokenEscape::Newline => Err(ErrorKind::UnexpectedNewline)?,
                TokenEscape::Prime => Err(ErrorKind::InvalidCharacterTooLess)?,
                TokenEscape::Quote(n) => match n {
                    0 => '"',
                    _ => Err(ErrorKind::InvalidCharacterTooMany)?,
                },
                TokenEscape::NoEscapeUtf8 | TokenEscape::NoEscapeAscii => {
                    let mut cs = tk.slice().chars();
                    let ch = cs.next().unwrap();
                    if cs.next().is_some() {
                        Err(ErrorKind::InvalidCharacterTooMany)?
                    }
                    ch
                }
                TokenEscape::EscapeByte => Err(ErrorKind::InvalidAsciiEscape)?,
                TokenEscape::EscapeAscii => esc::ascii(&tk),
                TokenEscape::EscapeUnicode => esc::unicode(&tk)?,
            };

            if let Some(TokenEscape::Prime) = tk.next().transpose()? {
                lex.bump(1);
                return Ok(Literal::Char(ch));
            }
        }

        Err(ErrorKind::UnexpectedEof)
    }

    // IMPROVE: Is it possible to borrow a "normal string without escape"?
    pub(crate) fn string<'i>(lex: &mut Lexer<'i, Token<'i>>) -> LexerResult<Literal<'i>> {
        let mut tk = switch::<_, TokenEscape>(lex);
        let mut s = String::new();

        while let Some(t) = tk.next().transpose()? {
            lex.bump(tk.slice().len());
            match t {
                TokenEscape::Newline => Err(ErrorKind::UnexpectedNewline)?,
                TokenEscape::Prime => s.push('\''),
                TokenEscape::Quote(n) => match n {
                    0 => return Ok(Literal::String(s)),
                    _ => Err(ErrorKind::UnbalancedLiteralClose)?,
                },
                TokenEscape::NoEscapeUtf8 | TokenEscape::NoEscapeAscii => s.push_str(tk.slice()),
                TokenEscape::EscapeByte => Err(ErrorKind::InvalidAsciiEscape)?,
                TokenEscape::EscapeAscii => s.push(esc::ascii(&tk)),
                TokenEscape::EscapeUnicode => s.push(esc::unicode(&tk)?),
            }
        }

        Err(ErrorKind::UnexpectedEof)
    }

    pub(crate) fn raw_string<'i>(lex: &mut Lexer<'i, Token<'i>>, q: usize) -> LexerResult<Literal<'i>> {
        let j = lex.remainder();
        let mut tk = switch::<_, TokenNoEscape>(lex);
        let mut len = 0;

        while let Some(t) = tk.next().transpose()? {
            len += tk.slice().len();

            if let TokenNoEscape::Quote(n) = t {
                match n.cmp(&q) {
                    Ordering::Less => continue,
                    Ordering::Equal => {
                        lex.bump(len);
                        return Ok(Literal::Str(unsafe {
                            core::str::from_utf8_unchecked(j[..len - tk.slice().len()].as_bytes())
                        }));
                    }
                    Ordering::Greater => Err(ErrorKind::UnbalancedLiteralClose)?,
                }
            }
        }

        Err(ErrorKind::UnexpectedEof)
    }

    // IMPROVE: Is it possible to borrow a "normal bytes without escape"?
    pub(crate) fn bytes<'i>(lex: &mut Lexer<'i, Token<'i>>) -> LexerResult<Literal<'i>> {
        let mut tk = switch::<_, TokenEscape>(lex);
        let mut buf = ByteBuf::new();

        while let Some(t) = tk.next().transpose()? {
            lex.bump(tk.slice().len());
            match t {
                TokenEscape::Newline => Err(ErrorKind::UnexpectedNewline)?,
                TokenEscape::Prime => buf.push(b'\''),
                TokenEscape::Quote(n) => match n {
                    0 => return Ok(Literal::ByteBuf(buf)),
                    _ => Err(ErrorKind::UnbalancedLiteralClose)?,
                },
                TokenEscape::NoEscapeUtf8 => Err(ErrorKind::UnexpectedNonAscii)?,
                TokenEscape::NoEscapeAscii => buf.extend_from_slice(tk.slice().as_bytes()),
                TokenEscape::EscapeByte => buf.push(esc::byte(&tk)),
                TokenEscape::EscapeAscii => buf.push(esc::ascii(&tk) as u8),
                TokenEscape::EscapeUnicode => Err(ErrorKind::UnexpectedUnicodeEscape)?,
            }
        }

        Err(ErrorKind::UnexpectedEof)
    }

    pub(crate) fn raw_bytes<'i>(lex: &mut Lexer<'i, Token<'i>>, q: usize) -> LexerResult<Literal<'i>> {
        let j = lex.remainder();
        let mut tk = switch::<_, TokenNoEscape>(lex);
        let mut len = 0;

        while let Some(t) = tk.next().transpose()? {
            len += tk.slice().len();

            match t {
                TokenNoEscape::Quote(n) => match n.cmp(&q) {
                    Ordering::Less => continue,
                    Ordering::Equal => {
                        lex.bump(len);
                        return Ok(Literal::Bytes(j[..len - tk.slice().len()].as_bytes()));
                    }
                    Ordering::Greater => Err(ErrorKind::UnbalancedLiteralClose)?,
                },
                TokenNoEscape::NoEscapeUtf8 => Err(ErrorKind::UnexpectedNonAscii)?,
                TokenNoEscape::NoEscapeAscii => continue,
                TokenNoEscape::__ => unreachable!(),
            }
        }

        Err(ErrorKind::UnexpectedEof)
    }

    pub(crate) fn bytes_encoding<'i>(lex: &mut Lexer<'i, Token<'i>>, flavor: BaseXX) -> LexerResult<Literal<'i>> {
        let j = lex.remainder();
        match j.find('"') {
            Some(n) => {
                lex.bump(n + 1);
                let content = j[..n].as_bytes();
                let base_err = |e| ErrorKind::InvalidBytesEncoding(e);
                return Ok(Literal::ByteBuf(match flavor {
                    BaseXX::Base16 => HEXUPPER_PERMISSIVE.decode(content).map_err(base_err)?,
                    BaseXX::Base32 => BASE32_NOPAD.decode(content).map_err(base_err)?,
                    BaseXX::Base64 => BASE64URL_NOPAD.decode(content).map_err(base_err)?,
                }));
            }
            None => Err(ErrorKind::UnexpectedEof)?,
        }
    }

    pub(crate) fn paragraph<'i>(lex: &mut Lexer<'i, Token<'i>>) -> LexerResult<Literal<'i>> {
        fn trim(mut s: &str) -> &str {
            s = &s.trim()[1..];
            s.strip_prefix('\x20').unwrap_or(s)
        }

        let first = trim(lex.slice());
        let mut tk = switch::<_, TokenParagraph>(lex);
        let mut newlined = false;

        Ok(match tk.next().transpose()? {
            Some(TokenParagraph::Leave) | None => Literal::Str(first),
            Some(mut t) => {
                let mut s = String::from(first);
                loop {
                    match t {
                        TokenParagraph::Leave => break,
                        t => {
                            lex.extras.borrow_mut().line += 1;
                            lex.bump(tk.slice().len());
                            let line = trim(tk.slice());

                            match t {
                                TokenParagraph::Leave => unreachable!(),
                                TokenParagraph::AsIsNewLine => {
                                    newlined = line.is_empty();
                                    s.push('\n');
                                    s.push_str(line);
                                }
                                TokenParagraph::JoinedLine | TokenParagraph::SpaceJoinedLine => match line.is_empty() {
                                    true => {
                                        if !newlined {
                                            newlined = true;
                                            s.push('\n');
                                        }
                                    }
                                    false => {
                                        if let TokenParagraph::SpaceJoinedLine = t {
                                            if !newlined {
                                                s.push('\x20');
                                            }
                                        }
                                        newlined = false;
                                        s.push_str(line);
                                    }
                                },
                            }
                        }
                    }

                    match tk.next().transpose()? {
                        Some(t_) => t = t_,
                        None => break,
                    }
                }
                Literal::String(s)
            }
        })
    }
}

mod esc {
    use super::*;

    /// `\xFF` - Includes leading backslash.
    pub(crate) fn byte(lex: &Lexer<TokenEscape>) -> u8 {
        let i = lex.slice().as_bytes();
        parse_with_options::<_, NUMBER_FMT_HEX>(&i[2..], PARSE_OPTS_INT).unwrap()
    }

    /// `\x7F` - Includes leading backslash.
    pub(crate) fn ascii(lex: &Lexer<TokenEscape>) -> char {
        let i = lex.slice().as_bytes();
        match i[1] {
            b'\\' => '\\',
            b'\"' => '\"',
            b'\'' => '\'',
            b'n' => '\n',
            b't' => '\t',
            b'r' => '\r',
            b'0' => '\0',
            b'x' => char::from_u32(parse_with_options::<_, NUMBER_FMT_HEX>(&i[2..], PARSE_OPTS_INT).unwrap()).unwrap(),
            _ => unreachable!(),
        }
    }

    /// `\u{3000}` - Includes leading backslash.
    pub(crate) fn unicode(lex: &Lexer<TokenEscape>) -> LexerResult<char> {
        let i = lex.slice().as_bytes();
        parse_with_options::<_, NUMBER_FMT_HEX>(&i[3..i.len() - 1], PARSE_OPTS_INT)
            .ok()
            .and_then(char::from_u32)
            .ok_or(ErrorKind::InvalidUnicodeEscape)
    }
}
