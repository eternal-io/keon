# KEON Grammar

> *KEON* : *Value* ( `;` *Value* )<sup>\*</sup> `;`<sup>?</sup>
>
> *Value* : [LITERAL](#literals) | [*Expressions*](#expressions)


## Notation

See [The Rust Reference](https://doc.rust-lang.org/stable/reference/notation.html).


## Lexical structure

TODO: whitespace and comments


### Keywords

> KW_TRUE : `true`      <br/>
> KW_FALSE : `false`    <br/>
> KW_INF : `inf`        <br/>
> KW_NAN : `NaN`


### Identifiers

> IDENT :
> <br/>&#x3000; NON_KEYWORD_IDENT | RAW_IDENT
>
> NON_KEYWORD_IDENT :
> <br/>&#x3000; IDENT_OR_KEYWORD <sub>*Except [keywords](#keywords)*</sub>
>
> RAW_IDENT :
> <br/>&#x3000; `` ` `` IDENT_OR_KEYWORD
>
> IDENT_OR_KEYWORD :
> <br/>&#x3000; XID_START XID_CONTINUE<sup>\*</sup>
> <br/>&#xFF5C; `_` XID_CONTINUE<sup>\+</sup>


## Literals

> LITERAL :
> <br/>&#x3000; [BOOLEAN_LITERAL](#boolean-literals)
> <br/>&#xFF5C; [INTEGER_LITERAL](#integer-literals)
> <br/>&#xFF5C; [FLOAT_LITERAL](#float-literals)
> <br/>&#xFF5C; [CHAR_LITERAL](#character-literals)
> <br/>&#xFF5C; [STRING_LITERAL](#string-literals)
> <br/>&#xFF5C; [BYTE_LITERAL](#byte-literals)
> <br/>&#xFF5C; [BYTE_STRING_LITERAL](#byte-string-literals)
> <br/>&#xFF5C; PARAGRAPH_LITERAL


### Boolean literals

> BOOLEAN_LITERAL :
> <br/>&#x3000; KW_TRUE | KW_FALSE


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
> HEX_DIGIT : \[`0`-`9` `A`-`F` `a`-`f`\]

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

> COM_ESCAPE :
> <br/>&#x3000; `\` \[`\` `"` `'` `0` `n` `t` `r`\]
>
> BYTE_ESCAPE :
> <br/>&#x3000; `\x` HEX_DIGIT<sup>2</sup>
>
> RICH_ESCAPE :
> <br/>&#x3000; `\x` OCT_DIGIT HEX_DIGIT | `\u{` ( HEX_DIGIT `_`<sup>\*</sup> )<sup>1..6</sup> `}`
>
> STRING_CONTINUE :
> <br/>&#x3000; `\` *followed by* \n

#### Character literals

> CHAR_LITERAL :
> <br/>&#x3000; `'` ( \~\[`'` `\` \\n \\t \\r\] | COM_ESCAPE | RICH_ESCAPE ) `'`

#### String literals

> STRING_LITERAL :
> <br/>&#x3000; COM_STRING_LITERAL
> <br/>&#xFF5C; RAW_STRING_LITERAL
>
> COM_STRING_LITERAL :
> <br/>&#x3000; `"` (
> <br/>&#x3000;&#x3000;&#x3000; \~\[`"` `\` \\r\]
> <br/>&#x3000;&#x3000;&#xFF5C; COM_ESCAPE
> <br/>&#x3000;&#x3000;&#xFF5C; RICH_ESCAPE
> <br/>&#x3000;&#x3000;&#xFF5C; STRING_CONTINUE
> <br/>&#x3000; )<sup>\*</sup> `"`
>
> RAW_STRING_LITERAL :
> <br/>&#x3000; `` ` ``<sup>k=1..255</sup> `"` ( \~\[\r\] )<sup>*(non-greedy)</sup> `"` `` ` ``<sup>k</sup>

#### Byte literals

> BYTE_LITERAL :
> <br/>&#x3000; `b'` ( \~\[`'` `\` \\n \\t \\r non-ASCII\] | COM_ESCAPE | BYTE_ESCAPE ) `'`

#### Byte string literals

> BYTE_STRING_LITERAL :
> <br/>&#x3000; COM_BYTE_STRING_LITERAL
> <br/>&#xFF5C; RAW_BYTE_STRING_LITERAL
> <br/>&#xFF5C; BASE16_BYTE_STRING_LITERAL
> <br/>&#xFF5C; BASE32_BYTE_STRING_LITERAL
> <br/>&#xFF5C; BASE64_BYTE_STRING_LITERAL
>
> COM_BYTE_STRING_LITERAL :
> <br/>&#x3000; `b"` (
> <br/>&#x3000;&#x3000;&#x3000; \~\[`"` `\` \\r *non-ASCII*\]
> <br/>&#x3000;&#x3000;&#xFF5C; COM_ESCAPE
> <br/>&#x3000;&#x3000;&#xFF5C; BYTE_ESCAPE
> <br/>&#x3000;&#x3000;&#xFF5C; STRING_CONTINUE
> <br/>&#x3000; )<sup>\*</sup> `"`
>
> RAW_BYTE_STRING_LITERAL :
> <br/>&#x3000; `b` `` ` ``<sup>k=1..255</sup> `"` ( \~[\r *non-ASCII*] )<sup>*(non-greedy)</sup> `"` `` ` ``<sup>k</sup>
>
> BASE16_BYTE_STRING_LITERAL :
> <br/>&#x3000; `b16"` HEX_DIGIT<sup>\*</sup> `"`
>
> BASE32_BYTE_STRING_LITERAL :
> <br/>&#x3000; `b32"` \[`A`-`Z` `2`-`7` `=`\]<sup>\*</sup> `"`
>
> BASE64_BYTE_STRING_LITERAL :
> <br/>&#x3000; `b64"` \[`A`-`Z` `a`-`z` `0`-`9` `-` `_` `=`\]<sup>\*</sup> `"`


## Expressions
