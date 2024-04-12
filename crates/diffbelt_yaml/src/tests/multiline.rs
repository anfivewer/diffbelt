use crate::parse_yaml;

#[test]
fn test_multiline() {
    let config = r#"
multiline: |
  test
  passed
"#;

    let docs = parse_yaml(config).expect("parsed");

    assert_eq!(docs.len(), 1);
    let doc = docs.get(0).expect("docs");

    let mapping = doc.as_mapping().expect("mapping");

    for (key, value) in mapping {
        let key = key.as_str().expect("str");
        let value = value.as_str().expect("value");

        assert_eq!(key, "multiline");
        assert_eq!(value, "test\npassed\n")
    }
}