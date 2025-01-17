mod util;
use serde::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Unary(u32);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Enum {
    Nullary,
    Unary(u32),
}

#[test]
#[rustfmt::skip]
fn roundtrips() {
    util::rt_min( &(0,)           , "(0,)"    ).unwrap();
    util::rt_min( &Unary(0)       , "%0"      ).unwrap();
    util::rt_min( &Enum::Unary(0) , "Unary%0" ).unwrap();

    util::rt_pre( &(0,)           , "(\n    0,\n)"    ).unwrap();
    util::rt_pre( &Unary(0)       , "(Unary)(0)"     ).unwrap();
    util::rt_pre( &Enum::Unary(0) , "Enum::Unary(0)" ).unwrap();

    util::rt_min( &Option::<()>::None , "?"   ).unwrap();
    util::rt_min( &Some(0)            , "?0"  ).unwrap();
    util::rt_pre( &Option::<()>::None , "?"   ).unwrap();
    util::rt_pre( &Some(0)            , "? 0" ).unwrap();

    util::rt_min( &((0,),)           , "((0,),)"    ).unwrap();
    util::rt_min( &(Unary(0),)       , "(%0,)"      ).unwrap();
    util::rt_min( &(Enum::Nullary,)  , "(Nullary,)" ).unwrap();
    util::rt_min( &(Enum::Unary(0),) , "(Unary%0,)" ).unwrap();

    util::rt_pre( &((0,),)           , "(\n    (\n        0,\n    ),\n)" ).unwrap();
    util::rt_pre( &(Unary(0),)       , "(\n    (Unary)(0),\n)"          ).unwrap();
    util::rt_pre( &(Enum::Nullary,)  , "(\n    Enum::Nullary,\n)"        ).unwrap();
    util::rt_pre( &(Enum::Unary(0),) , "(\n    Enum::Unary(0),\n)"      ).unwrap();
}

#[test]
fn backwards() {
    util::backward(&Unary(0), " % 0").unwrap();
    util::backward(&Unary(0), "()(0)").unwrap();
    util::backward(&Unary(0), "()(0,)").unwrap();
    util::backward(&Unary(0), "() % 0").unwrap();
    util::backward(&Unary(0), "(arbit)(0)").unwrap();
    util::backward(&Unary(0), "(arbit)(0,)").unwrap();
    util::backward(&Unary(0), "(arbit) % 0").unwrap();

    util::backward(&Enum::Unary(0), "Unary(0)").unwrap();
    util::backward(&Enum::Unary(0), "Unary(0,)").unwrap();
    util::backward(&Enum::Unary(0), "Unary % 0").unwrap();
    util::backward(&Enum::Unary(0), "arbit::Unary(0)").unwrap();
    util::backward(&Enum::Unary(0), "arbit::Unary(0,)").unwrap();
    util::backward(&Enum::Unary(0), "arbit::Unary % 0").unwrap();
}
