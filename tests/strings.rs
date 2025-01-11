mod util;

#[test]
#[rustfmt::skip]
fn roundtrips() {
    util::rt_min( &String::from("")         , r#""""#         ).unwrap();
    util::rt_min( &String::from("Test")     , r#""Test""#     ).unwrap();
    util::rt_min( &String::from("我测")     , r#""我测""#     ).unwrap();
    util::rt_min( &String::from("\n\t\r\0") , r#""\n\t\r\0""# ).unwrap();
    util::rt_min( &String::from("\x11\x23") , "\"\\x11#\""  ).unwrap();
}

#[test]
fn backwards() {
    util::backward(
        &String::from("expand 32-byte k"),
        r#""\x65\x78\x70\x61\x6e\x64\x20\x33\x32\x2d\x62\x79\x74\x65\x20\x6b""#,
    )
    .unwrap();
    util::backward(
        &String::from("expand 32-byte k"),
        r#""\x65\x78\x70\x61\x6E\x64\x20\x33\x32\x2D\x62\x79\x74\x65\x20\x6B""#,
    )
    .unwrap();
    util::backward(&String::from(r#"\1\2\3\x``"#), r#"`"\1\2\3\x``"`"#).unwrap();
    util::backward(&String::from(r#"\1\2\3``"`"#), r#"``"\1\2\3``"`"``"#).unwrap();
}
