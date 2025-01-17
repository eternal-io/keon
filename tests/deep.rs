use keon::Value;

#[test]
fn deep_object() {
    assert_eq!(
        keon::from_str::<Value>(&"?".repeat(10000)).unwrap_err().kind,
        keon::ErrorKind::ExceededRecursionLimit
    );

    assert_eq!(
        keon::from_str::<Value>(&"%".repeat(10000)).unwrap_err().kind,
        keon::ErrorKind::ExceededRecursionLimit
    );
}
