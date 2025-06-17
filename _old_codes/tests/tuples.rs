mod util;

#[test]
fn roundtrips() {
    util::rt_min(&(0,), "(0,)").unwrap();
    util::rt_min(&(0, 1), "(0,1)").unwrap();
    util::rt_min(&(0, 1, 2), "(0,1,2)").unwrap();
    util::rt_min(&(0, 1, 2, 3), "(0,1,2,3)").unwrap();

    util::rt_pre(&(0,), "(\n    0,\n)").unwrap();
    util::rt_pre(&(0, 1), "(\n    0,\n    1,\n)").unwrap();
    util::rt_pre(&(0, 1, 2), "(\n    0,\n    1,\n    2,\n)").unwrap();
    util::rt_pre(&(0, 1, 2, 3), "(\n    0,\n    1,\n    2,\n    3,\n)").unwrap();
}
