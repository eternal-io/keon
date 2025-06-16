# KEON Grammar


## Notation

TODO.


## Lexical structure

```go
KEON -> Value ( `;` Value )* `;`

Value ->
      LITERAL
    | ArrayExpression
    | OptionExpression
    | StructuralExpression
    | StructLikeExpression
    | EnumVariantExpression
```

TODO: whitespace.


### Comments

```go
COMMENT ->
    LINE_COMMENT | BLOCK_COMMENT

LINE_COMMENT ->
    `//` ( ~[<LF> <CR>] )*

BLOCK_COMMENT ->
    `/*` ( ~`*/` | BLOCK_COMMENT )* `*/`
```

Comments are interpreted as a form of whitespace.


### Keywords

```go
KW_TRUE  -> `true`
KW_FALSE -> `false`
KW_INF   -> `inf`
KW_NAN   -> `NaN`
```


### Identifiers

```go
IDENTIFIER ->
    NON_KEYWORD_IDENT | RAW_IDENT

NON_KEYWORD_IDENT ->
    IDENT_OR_KEYWORD !!except keywords

RAW_IDENT ->
    <BACKTICK> IDENT_OR_KEYWORD

IDENT_OR_KEYWORD ->
      XID_START XID_CONTINUE*
    | `_` XID_CONTINUE+
```


## Literals

```go
LITERAL ->
      BOOLEAN_LITERAL
    | INTEGER_LITERAL
    | FLOAT_LITERAL
    | CHAR_LITERAL
    | STRING_LITERAL
    | BYTE_LITERAL
    | BYTE_STRING_LITERAL
    | PARAGRAPH_LITERAL
```


### Boolean literals

```go
BOOLEAN_LITERAL ->
    KW_TRUE | KW_FALSE
```


### Number literals

#### Integer literals

```go
INTEGER_LITERAL ->
    DEC_LITERAL | BIN_LITERAL | OCT_LITERAL | HEX_LITERAL

DEC_LITERAL -> DEC_DIGIT ( `_` | DEC_DIGIT )*
BIN_LITERAL -> `0b` `_`* BIN_DIGIT ( `_` | BIN_DIGIT )*
OCT_LITERAL -> `0o` `_`* OCT_DIGIT ( `_` | OCT_DIGIT )*
HEX_LITERAL -> `0x` `_`* HEX_DIGIT ( `_` | HEX_DIGIT )*

BIN_DIGIT -> [`0`-`1`]
OCT_DIGIT -> [`0`-`7`]
DEC_DIGIT -> [`0`-`9`]
HEX_DIGIT -> [`0`-`9` `A`-`F` `a`-`f`]
```

#### Float literals

```go
FLOAT_LITERAL ->
      KW_INF
    | KW_NAN
    | DEC_LITERAL `.`
    | DEC_LITERAL ( `.` DEC_LITERAL )? FLOAT_EXPONENT

FLOAT_EXPONENT ->
    ( `e` | `E` ) ( `+` | `-` )? `_`* DEC_LITERAL
```


### Text literals

```go
COMMON_ESCAPE ->
    `\` [`\` `"` `'` `0` `n` `t` `r`]

BYTE_ESCAPE ->
    `\x` HEX_DIGIT{2}

CHAR_ESCAPE ->
    `\x` OCT_DIGIT HEX_DIGIT | `\u{` ( HEX_DIGIT `_`* ){1..6} `}`

STRING_CONTINUE ->
    `\` <LF>
```

#### Character literals

```go
CHAR_LITERAL ->
    `'` ( ~[`'` `\` <LF> <CR> <TAB>] | COMMON_ESCAPE | CHAR_ESCAPE ) `'`
```

#### String literals

```go
STRING_LITERAL ->
      COMMON_STRING_LITERAL
    | RAW_STRING_LITERAL

COMMON_STRING_LITERAL ->
    `"` (
          ~[`"` `\` <CR>]
        | COMMON_ESCAPE
        | CHAR_ESCAPE
        | STRING_CONTINUE
    )* `"`

RAW_STRING_LITERAL ->
    <BACKTICK>{k<-1..255} `"` ( ~[<CR>] )*? `"` <BACKTICK>{k}
```

#### Byte literals

```go
BYTE_LITERAL ->
    `b'` ( ~[`'` `\` <LF> <CR> <TAB> <non-ASCII>] | COMMON_ESCAPE | BYTE_ESCAPE ) `'`
```

#### Byte string literals

```go
BYTE_STRING_LITERAL ->
      COMMON_BYTE_STRING_LITERAL
    | RAW_BYTE_STRING_LITERAL
    | BASE16_BYTE_STRING_LITERAL
    | BASE32_BYTE_STRING_LITERAL
    | BASE64_BYTE_STRING_LITERAL

COMMON_BYTE_STRING_LITERAL ->
    `b"` (
          ~[`"` `\` <CR> <non-ASCII>]
        | COMMON_ESCAPE
        | BYTE_ESCAPE
        | STRING_CONTINUE
    )* `"`

RAW_BYTE_STRING_LITERAL ->
    `b` <BACKTICK>{k<-1..255} `"` ( ~[<CR> <non-ASCII>] )*? `"` <BACKTICK>{k}

BASE16_BYTE_STRING_LITERAL ->
    `b16"` HEX_DIGIT* `"`

BASE32_BYTE_STRING_LITERAL ->
    `b32"` [`A`-`Z` `2`-`7` `=`]* `"`

BASE64_BYTE_STRING_LITERAL ->
    `b64"` [`A`-`Z` `a`-`z` `0`-`9` `-` `_` `=`]* `"`
```


### Paragraph literals

```go
PARAGRAPH_LITERAL ->
    PARAGRAPH_START ( <LF> ( WHITE_SPACE !!except <LF> <CR> )* PARAGRAPH_CONTINUE )*

PARAGRAPH_START ->
    `|` <SPACE>? ( ~[<LF> <CR>] )*

PARAGRAPH_CONTINUE ->
    [`<` `|` `>`] <SPACE>? ( ~[<LF> <CR>] )*
```


## Expressions

```go
TupleExpression ->
    `(` ( ( Value `,` )+ Value? )? `)`

ArrayExpression ->
    `[` ( Value ( `,` Value )* `,`? )? `]`

OptionExpression ->
      `?` Value?

MayaryExpression ->
      `%` Value?

MapExpression ->
    `{` ( MapPair ( `,` MapPair )*  `,`? )? `}`

MapPair ->
      IDENTIFIER `:` Value
    | Value `=>` Value

StructuralExpression ->
      TupleExpression
    | MayaryExpression
    | MapExpression

StructLikeExpression ->
    `(` IDENTIFIER `)` StructuralExpression?

EnumVariantExpression ->
    ( IDENTIFIER `::` )? IDENTIFIER StructuralExpression?
```
