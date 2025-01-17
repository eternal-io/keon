mod util;
use serde::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Enum {
    Unit,
    Newtype(Box<Enum>),
    Tuple(i32, i32, i32),
    Struct { a: i32, b: i32 },
}

#[test]
fn roundtrips() {
    util::rt_min(&Enum::Unit, "Unit").unwrap();
    util::rt_min(&Enum::Newtype(Box::new(Enum::Unit)), "Newtype%Unit").unwrap();
    util::rt_min(&Enum::Tuple(1, 2, 3), "Tuple(1,2,3)").unwrap();
    util::rt_min(&Enum::Struct { a: 1, b: 2 }, "Struct{a:1,b:2}").unwrap();

    util::rt_pre(&Enum::Unit, "Enum::Unit").unwrap();
    util::rt_pre(&Enum::Newtype(Box::new(Enum::Unit)), "Enum::Newtype % Enum::Unit").unwrap();
    util::rt_pre(&Enum::Tuple(1, 2, 3), "Enum::Tuple(\n    1,\n    2,\n    3,\n)").unwrap();
    util::rt_pre(&Enum::Struct { a: 1, b: 2 }, "Enum::Struct {\n    a: 1,\n    b: 2,\n}").unwrap();
}
