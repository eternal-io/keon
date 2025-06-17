mod util;

#[test]
fn roundtrips() {
    util::rt_min(&'a', "'a'").unwrap();
    util::rt_min(&'\n', "'\\n'").unwrap();
    util::rt_min(&'\0', "'\\0'").unwrap();
    util::rt_min(&'\x08', "'\\x08'").unwrap();
    util::rt_min(&'\u{11}', "'\\x11'").unwrap();
    util::rt_min(&'\u{3000}', "'\u{3000}'").unwrap();
    util::rt_min(&'\u{2731}', "'✱'").unwrap();
    util::rt_min(&'✱', "'✱'").unwrap();
}
