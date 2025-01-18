use keon::Value;
use serde::Deserialize;

#[test]
fn deser_offset() {
    let input = r#""zxcv" !! 1123. !! ('a', 'b') !!"#;

    let mut der = keon::Deserializer::from_str(input);
    assert_eq!(Value::deserialize(&mut der).unwrap(), "zxcv".into());
    assert_eq!(der.offset(), 6);

    let mut der = keon::Deserializer::from_str(&input[6 + 3..]);
    assert_eq!(Value::deserialize(&mut der).unwrap(), 1123f64.into());
    assert_eq!(der.offset(), 6);

    let mut der = keon::Deserializer::from_str(&input[6 + 3 + 6 + 3..]);
    assert_eq!(
        Value::deserialize(&mut der).unwrap(),
        vec![Value::from('a'), Value::from('b')].into()
    );
    assert_eq!(der.offset(), 11);
}
