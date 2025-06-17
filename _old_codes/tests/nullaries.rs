mod util;
use serde::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct StructUnit;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TupleStructUnit();

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Enum {
    Unit,
    TupleUnit(),
    StructUnit {},
}

#[test]
#[rustfmt::skip]
fn roundtrips() {
    util::rt_min( &()                  , "()"           ).unwrap();
    util::rt_min( &StructUnit          , "()"           ).unwrap();
    util::rt_min( &TupleStructUnit()   , "%"            ).unwrap();
    util::rt_min( &Enum::Unit          , "Unit"         ).unwrap();
    util::rt_min( &Enum::TupleUnit()   , "TupleUnit%"   ).unwrap();
    util::rt_min( &Enum::StructUnit {} , "StructUnit{}" ).unwrap();

    util::rt_pre( &()                  , "()"                  ).unwrap();
    util::rt_pre( &StructUnit          , "(StructUnit)"        ).unwrap();
    util::rt_pre( &TupleStructUnit()   , "(TupleStructUnit)%"  ).unwrap();
    util::rt_pre( &Enum::Unit          , "Enum::Unit"          ).unwrap();
    util::rt_pre( &Enum::TupleUnit()   , "Enum::TupleUnit%"    ).unwrap();
    util::rt_pre( &Enum::StructUnit {} , "Enum::StructUnit {}" ).unwrap();
}
