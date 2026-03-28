# KEON Grammar

- Definitions named with `SCREAMING_SNAKE_CASE` are *atoms*.
- Definitions named with `UpperCamelCase` are *compounds*.
- WHITESPACE and COMMENTs are allowed only **between** definitions in a *compound*.

#### Specials

- `<LF>`: `U+000A` (line feed `'\n'`)
- `<CR>`: `U+000D` (carriage return `'\r'`)
- `<TAB>`: `U+0009` (horizontal tab `'\t'`)
- `<SPACE>`: `U+0020` (space `' '`)
- `<BACKTICK>`: `U+0060` (grave accent ``'`'``)
- `<Non-ASCII>`: Non-ASCII characters
- `<EOF>` : end of input
- `XID_Start` and `XID_Continue`: as defined in [Unicode Standard Annex #31](https://www.unicode.org/reports/tr31/tr31-41.html)

---

```GO
/*== Overall ==*/

Keon -> KeonPartial+
  // == Value ( `;` Value )* `;`? <EOF>

KeonPartial -> Value ( `;` | `;`? <EOF> )

Value ->
      LITERAL
    | Structure
    | NominalStructure


/*== Whitespace ==*/

WS -> ( WHITESPACE* COMMENT )* WHITESPACE*

WHITESPACE -> !!characters that have `White_Space` Unicode property

/* U+0009 (horizontal tab, '\t')
 * U+000A (line feed, '\n')
 * U+000B (vertical tab)
 * U+000C (form feed)
 * U+000D (carriage return, '\r')
 * U+0020 (space, ' ')
 * U+0085 (next line)
 * U+200E (left-to-right mark)
 * U+200F (right-to-left mark)
 * U+2028 (line separator)
 * U+2029 (paragraph separator)
 */

/*== Comments ==*/

COMMENT ->
    LINE_COMMENT | BLOCK_COMMENT

LINE_COMMENT ->
    `//` ( ~<LF> )*

BLOCK_COMMENT ->
    `/*` ( BLOCK_COMMENT | ~`*/` )* `*/`


/*== Keywords ==*/

KW_TRUE -> `true`
KW_FALSE -> `false`
KW_INFINITY -> `inf`
KW_NOTANUMBER -> `NaN`

/*== Identifiers ==*/

IDENTIFIER ->
    NON_KEYWORD_IDENT | RAW_IDENT

NON_KEYWORD_IDENT ->
    IDENT_OR_KEYWORD !!except keyword

RAW_IDENT ->
    <BACKTICK> IDENT_OR_KEYWORD

IDENT_OR_KEYWORD ->
      XID_Start XID_Continue*
    |       `_` XID_Continue+


/*== Literals ==*/

LITERAL ->
      BOOLEAN_LITERAL
    | INTEGER_LITERAL
    | FLOAT_LITERAL
    | CHAR_LITERAL
    | STRING_LITERAL
    | BYTE_LITERAL
    | BYTE_STRING_LITERAL
    | PARAGRAPH_LITERAL

// boolean literals
BOOLEAN_LITERAL ->
    `true` | `false`

// integer literals
INTEGER_LITERAL ->
    `-`? ( DEC_LITERAL | BIN_LITERAL | OCT_LITERAL | HEX_LITERAL ) INTEGER_SUFFIX?

INTEGER_SUFFIX ->
      `i8` | `i16` | `i32` | `i64` | `i128`
    | `u8` | `u16` | `u32` | `u64` | `u128`

DEC_LITERAL -> DEC_DIGIT ( `_` | DEC_DIGIT )*
BIN_LITERAL -> `0b` `_`* BIN_DIGIT ( `_` | BIN_DIGIT )*
OCT_LITERAL -> `0o` `_`* OCT_DIGIT ( `_` | OCT_DIGIT )*
HEX_LITERAL -> `0x` `_`* HEX_DIGIT ( `_` | HEX_DIGIT )*

BIN_DIGIT -> [`0`-`1`]
OCT_DIGIT -> [`0`-`7`]
DEC_DIGIT -> [`0`-`9`]
HEX_DIGIT -> [`0`-`9` `A`-`F` `a`-`f`]

// float literals
FLOAT_LITERAL ->
    `-`? ( `inf` | `NaN` | DEC_LITERAL ( `.` DEC_LITERAL? )? FLOAT_EXPONENT? ) FLOAT_SUFFIX?

FLOAT_EXPONENT ->
    ( `e` | `E` ) ( `+` | `-` )? `_`* DEC_LITERAL

FLOAT_SUFFIX ->
    `f32` | `f64`

NEWLINE ->
    <CR>? <LF>

ESCAPE_COMMON ->
    `\` [`\` `"` `'` `0` `n` `t` `r`]

ESCAPE_BYTE ->
    `\x` HEX_DIGIT{2}

ESCAPE_CHAR ->
    `\x` OCT_DIGIT HEX_DIGIT | `\u{` ( HEX_DIGIT `_`* ){1..=6} `}`

// character literals
CHAR_LITERAL ->
    `'` ( ~[`'` `\` <LF> <CR> <TAB>] | ESCAPE_COMMON | ESCAPE_CHAR ) `'`

// string literals
STRING_LITERAL ->
      STRING_LITERAL_NORMAL
    | STRING_LITERAL_RAW

STRING_LITERAL_NORMAL ->
    `"` ( ~[`"` `\` <CR>] | NEWLINE | ESCAPE_COMMON | ESCAPE_CHAR )* `"`

STRING_LITERAL_RAW ->
    <BACKTICK>{k<-1..} `"` ( ~<CR> | NEWLINE )*? `"` <BACKTICK>{k}

// byte literals
BYTE_LITERAL ->
    `b'` ( ~[`'` `\` <LF> <CR> <TAB> <Non-ASCII>] | ESCAPE_COMMON | ESCAPE_BYTE ) `'`

// byte string literals
BYTE_STRING_LITERAL ->
      BYTE_STRING_LITERAL_NORMAL
    | BYTE_STRING_LITERAL_RAW
    | BYTE_STRING_LITERAL_BASE16
    | BYTE_STRING_LITERAL_BASE32
    | BYTE_STRING_LITERAL_BASE64

BYTE_STRING_LITERAL_NORMAL ->
    `b"` ( ~[`"` `\` <CR> <Non-ASCII>] | NEWLINE | ESCAPE_COMMON | ESCAPE_BYTE )* `"`

BYTE_STRING_LITERAL_RAW ->
    `b` <BACKTICK>{k<-1..} `"` ( ~[<CR> <Non-ASCII>] | NEWLINE )*? `"` <BACKTICK>{k}

BYTE_STRING_LITERAL_BASE16 ->
    `b16"` HEX_DIGIT* `"`

BYTE_STRING_LITERAL_BASE32 ->
    `b32"` [`A`-`Z` `2`-`7` `=`]* `"`

BYTE_STRING_LITERAL_BASE64 ->
    `b64"` [`A`-`Z` `a`-`z` `0`-`9` `-` `_` `=`]* `"`

// paragraph literals
PARAGRAPH_LITERAL ->
    <BACKTICK>{k<-1..} `|` <SPACE>? ( ~[<LF> <CR>] )* (
        NEWLINE WS
        <BACKTICK>{k} [`|` `<` `>`] <SPACE>? ( ~[<LF> <CR>] )*
    )*          // ^ If there is an opportunity to do something "wrong", someone will do it.
                //   So I chose to simply limit the length of subsequent delimiter sequences
                //   to the same length as the first line, to keep it "correct".


/*== Structures ==*/

Structure ->
      MaybeValue
    | TupleValue
    | SeqValue
    | MapValue

MaybeValue ->
    `?` Value?

TupleValue ->
    `(` ( Value ( `,` Value )* `,`? )? `)`

SeqValue ->
    `[` ( Value ( `,` Value )* `,`? )? `]`

MapValue ->
    `{` (
              Value `=>` Value
        ( `,` Value `=>` Value )* `,`?
    )? `}`


/*== Nominal Structures ==*/

NominalStructure ->
    NominalPath ( TupleValue | StructValue )?

NominalPath ->
    ( `_` | IDENTIFIER ) ( `::` IDENTIFIER )?
    // We could certainly support complex paths like `path::to::Foo::Bar`,
    // but this seemed to lack usefulness and is no longer provided.

StructValue ->
    `{` (
              IDENTIFIER `:` Value
        ( `,` IDENTIFIER `:` Value )* `,`?
    )? `}`
```
