# KEON Grammar

> *KEON* : *Value* ( `;` *Value* )<sup>\*</sup> `;`<sup>?</sup>
>
> *Value* : [*Atom*](#atoms) | [*Compound*](#compounds)


## Notation

See [The Rust Reference](https://doc.rust-lang.org/stable/reference/notation.html).


## Lexical structure

（TODO：空格和注释在哪里加入）


## Tokens

### Keywords

> KW_TRUE : `true`      <br/>
> KW_FALSE : `false`    <br/>
> KW_INF : `inf`        <br/>
> KW_NAN : `NaN`


### Identifiers

> IDENT : NON_KEYWORD_IDENT | RAW_IDENT
>
> NON_KEYWORD_IDENT : IDENT_OR_KEYWORD <sub>Except [keywords](#keywords)</sub>
>
> RAW_IDENT : `` ` `` IDENT_OR_KEYWORD
>
> IDENT_OR_KEYWORD :
> <br/>&#x3000; XID_START XID_CONTINUE<sup>\*</sup>
> <br/>&#xFF5C; `_` XID_CONTINUE<sup>\+</sup>


## Atoms

> Atom :
> <br/>&#x3000; [BOOLEAN_LITERAL](#boolean-literals)
> <br/>&#xFF5C; [INTEGER_LITERAL](#integer-literals)
> <br/>&#xFF5C; [FLOAT_LITERAL](#float-literals)
> <br/>&#xFF5C; CHAR_LITERAL
> <br/>&#xFF5C; [STRING_LITERAL](#string-literals)
> <br/>&#xFF5C; RAW_STRING_LITERAL
> <br/>&#xFF5C; BYTES_LITERAL
> <br/>&#xFF5C; RAW_BYTES_LITERAL
> <br/>&#xFF5C; PARAGRAPH_LITERAL


### Boolean literals

> BOOLEAN_LITERAL :
> <br/>&#x3000; KW_TRUE
> <br/>&#xFF5C; KW_FALSE

### Number literals

#### Integer literals

> INTEGER_LITERAL :
> <br/>&#x3000; DEC_LITERAL | BIN_LITERAL | OCT_LITERAL | HEX_LITERAL
>
> DEC_LITERAL :
> <br/>&#x3000; DEC_DIGIT ( `_` | DEC_DIGIT )<sup>\*</sup>
>
> BIN_LITERAL :
> <br/>&#x3000; `0b` `_`<sup>\*</sup> BIN_DIGIT ( `_` | BIN_DIGIT )<sup>\*</sup>
>
> OCT_LITERAL :
> <br/>&#x3000; `0o` `_`<sup>\*</sup> OCT_DIGIT ( `_` | OCT_DIGIT )<sup>\*</sup>
>
> HEX_LITERAL :
> <br/>&#x3000; `0x` `_`<sup>\*</sup> HEX_DIGIT ( `_` | HEX_DIGIT )<sup>\*</sup>
>
>
> BIN_DIGIT : \[`0`-`1`\]
>
> OCT_DIGIT : \[`0`-`7`\]
>
> DEC_DIGIT : \[`0`-`9`\]
>
> HEX_DIGIT : \[`0`-`9` `a`-`f` `A`-`F`\]

#### Float literals

> FLOAT_LITERAL :
> <br/>&#x3000; KW_INF
> <br/>&#xFF5C; KW_NAN
> <br/>&#xFF5C; DEC_LITERAL `.`
> <br/>&#xFF5C; DEC_LITERAL ( `.` DEC_LITERAL )<sup>?</sup> FLOAT_EXPONENT
>
> FLOAT_EXPONENT :
> <br/>&#x3000; ( `e` | `E` ) ( `+` | `-` )<sup>?</sup> `_`<sup>\*</sup> DEC_LITERAL


### Text literals

> ESCAPE_BYTE : `\x` HEX_DIGIT<sup>2</sup>
>
> ESCAPE_ASCII : `\x` OCT_DIGIT HEX_DIGIT
>
> ESCAPE_NORMAL : `\` \[`\` `"` `'` `0` `n` `t` `r`\]
>
> ESCAPE_UNICODE : `\u{` ( HEX_DIGIT `_`<sup>\*</sup> )<sup>1..6</sup> `}`

#### Character literals

#### String literals

> STRING_LITERAL :
> <br/>&#x3000; `"` (
> <br/>&#x3000;&#x3000;&#x3000; \~\[`"` `\` \\r \]
> <br/>&#x3000;&#x3000;&#xFF5C; ESCAPE_ASCII
> <br/>&#x3000;&#x3000;&#xFF5C; ESCAPE_NORMAL
> <br/>&#x3000;&#x3000;&#xFF5C; ESCAPE_UNICODE
> <br/>&#x3000;&#x3000;&#xFF5C; STRING_CONTINUE
> <br/>&#x3000; )<sup>\*</sup> `"`
>
> STRING_CONTINUE :
> <br/>&#x3000; `\` *followed by* \n

#### Bytes literals


## Compounds
