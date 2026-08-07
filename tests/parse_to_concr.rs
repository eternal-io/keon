// use keon::parse;

// #[test]
// fn floats() {
//     assert_eq!(
//         (
//             f32::INFINITY,
//             f32::NEG_INFINITY,
//             -5.0f32,
//             3.14f32,
//             11.23f64,
//             10f32.powi(f32::MAX_10_EXP),
//             10f32.powi(f32::MIN_10_EXP),
//             10f64.powi(f64::MAX_10_EXP),
//             10f64.powi(f64::MIN_10_EXP),
//         ),
//         parse(
//             "(
//                 inf,    -inf,
//                 -5.0,   3.14,   11.23,
//                 1.0e38, 1.0e-37,1.0e308,1.0e-307,
//             )"
//         )
//         .unwrap()
//     );
// }
