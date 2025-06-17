mod util;
use serde::*;

#[allow(non_snake_case)]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct RawIdents {
    r#true: bool,
    r#false: bool,
    inf: f32,
    NaN: f64,
    Lim: (),
}

#[test]
fn roundtrips() {
    util::forward(
        &RawIdents {
            r#true: true,
            r#false: false,
            inf: f32::INFINITY,
            NaN: f64::NAN,
            Lim: (),
        },
        "{`true:true,`false:false,`inf:inf,`NaN:NaN,Lim:()}",
    )
    .unwrap();

    util::backward(
        &RawIdents {
            r#true: true,
            r#false: false,
            inf: f32::INFINITY,
            NaN: f64::NEG_INFINITY,
            Lim: (),
        },
        "{`true:true,`false:false,`inf:inf,`NaN:-inf,Lim:()}",
    )
    .unwrap();
}
