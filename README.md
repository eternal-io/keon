# KEON

[![](https://img.shields.io/crates/v/keon)](https://crates.io/crates/keon)
[![](https://img.shields.io/crates/d/keon)](https://crates.io/crates/keon)
[![](https://img.shields.io/crates/msrv/keon)](https://github.com/eternal-io/keon)
[![](https://img.shields.io/crates/l/keon)](#)
[![](https://img.shields.io/docsrs/keon)](https://docs.rs/keon)
[![](https://img.shields.io/github/stars/eternal-io/keon?style=social)](https://github.com/eternal-io/keon)


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
  - Distinguishable between tuples and lists (seqs).
  - Arbitrary type as dictionary (map) keys.
  - ...
- Shorthand for newtypes.
- Support use Base64, Base32 and Base16 to represent bytes.
- Provide paragraphs may be helpful when writing something by hand.


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
