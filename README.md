# KEON

[![](https://img.shields.io/crates/v/keon)](https://crates.io/crates/keon)
[![](https://img.shields.io/crates/d/keon)](https://crates.io/crates/keon)
[![](https://img.shields.io/crates/msrv/keon)](https://github.com/eternal-io/keon)
[![](https://img.shields.io/crates/l/keon)](#)
[![](https://img.shields.io/docsrs/keon)](https://docs.rs/keon)
[![](https://img.shields.io/github/stars/eternal-io/keon?style=social)](https://github.com/eternal-io/keon)


***🚧 This is the development branch. If you see this message, it means the following contents are still outdated. 🚧***


KEON is a human readable object notation / serialization format that syntactic similar to Rust and completely supports [Serde's data model](https://serde.rs/data-model.html).

<details><summary><b>📝 Cheat sheet</b></summary>

| Unit     | `()`
| --------:|:------ |
| Booleans | `true` , `false`
| Numbers  | `42` , `0x1123` , `-1` , `3.14` , `inf` , `NaN`
| Chars    | `'A'` , `'✱'` , `'\n'` , `'\u{3000}'`
| Strings  | `"Hello"` , <code>&#96;&#34;raw string \^o^/&#34;&#96;</code>
| Bytes    | `b"Hello"` , <code>b&#96;&#34;raw bytes \^o^/&#34;&#96;</code> , `b64"Sy0tQWV0aGlheA"`
| Options  | `?` , `? Thing`
| Tuples   | `(T,)` , `(T, U, V)`
| Lists    | `["abc", "def"]`
| Maps     | `{ 1 => 2, 3 => 4 }`
| Structs  | `(Struct) { field1: "value1", field2: "value2" }`
| Variants | `Enum::Variant` , `Variant`

And the Paragraphs, leave anything after the *start sign* of each line intact:

<table>
<tr><td align="right">As is newline</td><td>

```keon
| #include<iostream>
` using namespace std;
` int main() {
`     cout << "..." << endl;
`     return 0;
` }
```

</td></tr>
<tr><td align="right">Space-joined line<br /><sup>(will trim spaces)</sup></td><td>

```keon
| To be,
| or not to be,
| that is the question.
```

</td></tr>
<tr><td align="right">Joined line<br /><sup>(will trim spaces)</sup></td><td>

```keon
| 我能吞下
< 玻璃而不
< 伤身体。
```

</td></tr>
</table>

The start signs can be mixed, but the first must be the vertical-bar `|`.
</details>

> [!NOTE]
> This is not ready for production use, more comprehensive tests are needed, and there is no standard yet.


## [Example](https://github.com/eternal-io/keon/blob/master/examples/roundtrip.rs)

#### A simple struct in KEON:

```keon
(Save) {                            // <- optional struct name.
    greeting: "Hello world!",
    keybinds: {
        Action::Up => 'W',          // <- optional enum name.
        Action::Down => 'S',
        Action::Left => 'A',
        Action::Right => 'D',
    },
    difficulty_factor: 4.3,
    respawn_point: (                // <- can use parentheses `(` for tuple `)`.
        1107.1487,
        1249.0458,
    ),
    inventory: [
        Item::Water,
        Item::CannedFood,
        Item::IdCard > 101,         // <- newtype variant.
        Item::RocketLauncher {
            damage: 250,
            explosion_radius: 60.0,
        },
    ],
}
```

#### The same happens in JSON:

```json
{
  "greeting": "Hello world!",
  "keybinds": {
    "A": "Left",
    "D": "Right",
    "S": "Down",
    "W": "Up"
  },
  "difficulty_factor": 4.3,
  "respawn_point": [
    1107.1487,
    1249.0458
  ],
  "inventory": [
    "Water",
    "CannedFood",
    {
      "IdCard": 101
    },
    {
      "RocketLauncher": {
        "damage": 250,
        "explosion_radius": 60.0
      }
    }
  ]
}
```


## Our advantages

- Less syntactic noise, more intuitive look.
- Allow comments and trailing commas.
- Write KEON almost like you write Rust:
  - Humanized optional type annotation.
  - Arbitrary type as maps keys.
  - Use braces `{}` to represent maps and structs (RON doesn't).
  - Distinguishable between tuples and lists (though they're all `seq` in Serde).
  - ...
- Shorthand for newtypes.
- Supports use Base64, Base32 and Base16 to represent bytes.
- Provides Paragraph may be helpful when writing something by hand.


## Quick usage

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct MyData {
    msg: String,
    float: f32,
}

fn main() {
    let dat: MyData = keon::from_str(r#"{ msg: "Hello!", float: 1123. }"#).unwrap();

    println!("KEON: {}", keon::to_string(&dat).unwrap());

    println!("Pretty KEON: {}", keon::to_string_pretty(&dat).unwrap());
}
```


## Motivations

RON uses parentheses to represent structs. *"This is not Rusty at all!"*, I thought to myself.
This is where the story begins, a project written out of OCD. Eventually, KEON is different from RON in the following ways:

- Use braces `{}` to represent structs and maps.

- `macro_rules!` tells us: `expr` and `stmt` may only be followed by one of: `=>`, `,`, or `;`.

  RON uses only `:` even though the left-hand can be arbitrary. KEON has added `=>`, now we have two ways to represent key-to-value.
  This is why structs and maps can be unified: structs can be regarded as maps with strings as keys.
  `ident: ...` is basically syntactic sugar for `"ident" => ...`.

  However, these two ways are guaranteed NOT to be mixed in serialization output:
  Structs always use colons; Maps always use fat arrows, even though they use strings as keys.

- Since parentheses are saved, we can use `()` to represent tuples and `[]` to represent vectors.
  Although they are all `seq` in Serde, in the output, this certainty reassures me: the length of a tuple is immutable relative to a vector.

- Serde allows some weird structures, such as `struct AwfulNullary()`, which must `visit_tuple` rather than `visit_unit`.
  And `enum Foo { AwfulNullary() }`. Even though these never happened, I insisted on getting it sorted out.
  - In RON, the former outputs `()` when hiding struct names, while both output `AwfulNullary()` when showing struct names.
    Only backend knows its exact type, that's unsettling to me.
  - In KEON, pretty outputs `(AwfulNullary)()` and `Foo::AwfulNullary()`,
    or minimal outputs `()()` and `AwfulNullary()` respectively. You can tell what's going on at a glance.

- Variants can be written anywhere as `Enum::Variant` or just `Variant`, exactly as happens in Rust.
  Redundant annotations help to quickly figure out what's there, and jump to the corresponding location without relying too much on LSP?

- The type annotation of structs is done by `(TheStruct)`, like type conversions in C, implying the backend doesn't care what's in...
  If the parentheses were omitted, `TheStruct` would be treated as a variant in most places (refer to Turbofish), and I would not be able to write a usable parser at all.
  Although this isn't Rusty, it should not be too obtrusive.

- RON doesn't guarantee work with `deserialize_any` may have to do with these details.
  I believe KEON can support that, but more comprehensive tests are needed.

#### Some other less Rusty things:

- `Option<T>` doesn't accept `visit_enum`, it only accepts `visit_some`/`none`.
  I didn't want to provide exceptions for `Some(..)` and `None`, so I had to find the question mark `?` from my keyboard for it to use.

- Serde provides `visit_newtype_struct`, I think this *must* have its purpose, so we'd better have the corresponding syntactic sugar, that is `>`.
  Of course things like `Item::IdCard(101)` are also legal.

- Raw strings. KEON uses Backtick-Quote <code>&#96;&#96;&#34;...&#34;&#96;&#96;</code> instead of R-Pound-Quote `r#"..."#`.
  This is because, when I want to turn a string to a raw string, after selecting them, I can't wrap them by simply hitting `#` &mdash; they will be directly overwritten, this annoys me somethings.
  But backtick can almost always automatically enclose the selection without worrying about ambiguity, requires less typing, and is just as intuitive.

- Correspondingly, raw identifier uses backtick instead, such as <code>&#96;true</code> and <code>&#96;false</code>.

- Paragraphs, added purely out of preference. I wanted to try out how much handwriting would be benefited by providing this syntax, for a language that is indent-insensitive.

- Base64, Base32 and Base16. Fine, they are free.
