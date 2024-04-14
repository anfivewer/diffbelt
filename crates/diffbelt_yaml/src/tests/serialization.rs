use crate::parse_yaml;

#[test]
fn test_serialization() {
    let config = r#"multiline: |
  test
  passed
"#;

    let docs = parse_yaml(config).expect("parsed");

    assert_eq!(docs.len(), 1);
    let doc = docs.get(0).expect("docs");

    let mut output = String::new();

    () = doc.serialize(&mut output).expect("serialization error");

    assert_eq!(output.as_str(), config);
}
