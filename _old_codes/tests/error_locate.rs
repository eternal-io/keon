use keon::Value;

fn err_line_col(s: &str) -> String {
    let msg = keon::from_str::<Value>(s).unwrap_err().to_string();
    eprintln!("{}", msg);
    msg.split(' ').next().unwrap().to_string()
}

#[test]
fn deserialization() {
    assert_eq!(":1:1", err_line_col(""));
    assert_eq!(":1:5", err_line_col("asdf`"));
    assert_eq!(
        ":2:18",
        err_line_col(
            r#"{
            (foo)}"#
        )
    );
    assert_eq!(
        ":5:17",
        err_line_col(
            r#"
            // some comment
            {
                (foo) => /* unit */ (bar),
            }   quinn
            "#
        )
    );
    assert_eq!(
        ":2:-1",
        err_line_col(
            r#""broken!
            ...""#
        )
    );

    // Variants cannot made into Value via serde.
    // What's more, the located error of these are featured.
    // This is about peek, but shouldn't have much impact.
    assert_eq!(":1:11", err_line_col("after_this"));
    assert_eq!(":1:17", err_line_col("after_path_sep::before_this"));
}
