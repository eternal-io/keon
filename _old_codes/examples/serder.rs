use keon::{Deserializer, SerializeConfig, Serializer};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

fn main() {
    let time = SystemTime::now();

    let mut buf = Vec::<u8>::new();
    let mut ser = Serializer::new(&mut buf, SerializeConfig::comfort());
    time.serialize(&mut ser).expect("serialize");

    let s = String::from_utf8(buf).unwrap();
    println!("{}", s);

    let mut der = Deserializer::from_str(&s);
    let time_back = SystemTime::deserialize(&mut der).expect("deserialize");
    der.finish().expect("eof");

    assert_eq!(time, time_back);
}
