use std::time::SystemTime;

use keon::{value::Map, Value};

fn main() {
    let val = Value::from_str(
        "(SystemTime) {
            secs_since_epoch: 1736172788,
            nanos_since_epoch: 855221200,
        }",
    )
    .unwrap();

    let val2 = Value::Map(Map::from_iter(
        vec![
            ("secs_since_epoch".into(), 1736172788u64.into()),
            ("nanos_since_epoch".into(), 855221200u64.into()),
        ]
        .into_iter(),
    ));

    assert_eq!(val, val2);

    let _ = val.into_rust::<SystemTime>().unwrap();
}
