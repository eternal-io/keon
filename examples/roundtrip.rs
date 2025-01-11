use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Write},
};

fn main() -> io::Result<()> {
    let save = Save {
        greeting: "Hello world!".to_string(),
        keybinds: BTreeMap::from_iter(vec![
            (Action::Up, 'W'),
            (Action::Down, 'S'),
            (Action::Left, 'A'),
            (Action::Right, 'D'),
        ]),

        difficulty_factor: 4.3,
        respawn_point: (1107.148717794, 1249.045772398),

        inventory: vec![
            Item::Water,
            Item::CannedFood,
            Item::IdCard(101),
            Item::RocketLauncher {
                damage: 250,
                explosion_radius: 60.0,
            },
        ],
    };

    File::create("examples/roundtrip.keon")?
        .write_all(keon::to_string_pretty(&save).expect("serialization failed").as_bytes())?;

    let s = fs::read_to_string("examples/roundtrip.keon")?;
    println!("{}", s);

    let save_back = keon::from_str(&s).expect("deserialization failed");
    assert_eq!(save, save_back);

    println!(
        "\nCompare to JSON: \n\n{}",
        serde_json::to_string_pretty(&save).unwrap()
    );

    Ok(())
}

//------------------------------------------------------------------------------

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Save {
    greeting: String,
    keybinds: BTreeMap<Action, char>,

    difficulty_factor: f32,
    respawn_point: (f32, f32),

    inventory: Vec<Item>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum Action {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Item {
    Water,
    CannedFood,
    IdCard(u32),
    RocketLauncher { damage: i32, explosion_radius: f32 },
}
