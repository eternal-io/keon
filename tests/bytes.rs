mod util;
use serde::*;

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Bytes(#[serde(with = "serde_bytes")] Vec<u8>);

#[test]
fn roundtrips() {
    util::rt_pre(&Bytes(b"".to_vec()), r#"(Bytes) % b"""#).unwrap();
    util::rt_pre(
        &Bytes(b"\x01\x02\x21\x22\x7f\x80".to_vec()),
        "(Bytes) % b\"\\x01\\x02!\\\"\\x7f\\x80\"",
    )
    .unwrap();

    util::rt_min(&Bytes(b"".to_vec()), r#"%b64"""#).unwrap();
    util::rt_min(&Bytes(b"\x01\x02\x21\x22\x7f\x80".to_vec()), r#"%b64"AQIhIn-A""#).unwrap();
}

#[test]
fn backwards() {
    util::backward(
        &Bytes(b"\x01\x02\x21\x22\x7f\x80".to_vec()),
        r#"%b"\x01\x02!\"\x7f\x80""#,
    )
    .unwrap();
    util::backward(&Bytes(b"\x01\x02\x21\x22\x7f\x80".to_vec()), r#"%b64"AQIhIn-A""#).unwrap();
    util::backward(&Bytes(b"\x01\x02\x21\x22\x7f\x80".to_vec()), r#"%b32"AEBCCIT7QA""#).unwrap();
    util::backward(&Bytes(b"\x01\x02\x21\x22\x7f\x80".to_vec()), r#"%b16"010221227F80""#).unwrap();
}
