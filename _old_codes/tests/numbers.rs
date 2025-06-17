mod util;

#[test]
fn roundtrips() {
    util::rt_min(&0, "0").unwrap();
    util::rt_min(&1, "1").unwrap();
    util::rt_min(&-1, "-1").unwrap();
    util::rt_min(&i64::MIN, "-9223372036854775808").unwrap();
    util::rt_min(&u64::MAX, "18446744073709551615").unwrap();

    util::rt_min(&-5.0f32, "-5.0").unwrap();
    util::rt_min(&2.3333f32, "2.3333").unwrap();
    util::rt_min(&2.3333f64, "2.3333").unwrap();
    util::rt_min(&f32::INFINITY, "inf").unwrap();
    util::rt_min(&f32::NEG_INFINITY, "-inf").unwrap();
    util::rt_min(&10f32.powi(f32::MAX_10_EXP), "1.0e38").unwrap();
    util::rt_min(&10f64.powi(f64::MAX_10_EXP), "1.0e308").unwrap();
    util::rt_min(&10f32.powi(f32::MIN_10_EXP), "1.0e-37").unwrap();
    util::rt_min(&10f64.powi(f64::MIN_10_EXP), "1.0e-307").unwrap();
}
