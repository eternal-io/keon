use keon::parse;

#[test]
fn floats() {
    assert_eq!(
        (
            -5.0f32,
            11.23f32,
            11.23f64,
            f32::INFINITY,
            f32::NEG_INFINITY,
            10f32.powi(f32::MAX_10_EXP),
            10f32.powi(f32::MIN_10_EXP),
            10f64.powi(f64::MAX_10_EXP),
            10f64.powi(f64::MIN_10_EXP),
        ),
        parse(
            "(
            -5.0,
            11.23,
            11.23,
            inf,
            -inf,
            1.0e38,
            1.0e-37,
            1.0e308,
            1.0e-307,
        )"
        )
        .unwrap()
    );
}
