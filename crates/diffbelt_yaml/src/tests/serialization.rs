use crate::parse_yaml;

#[test]
fn test_serialization() {
    let config = r#"simple: value
multiline: >
  test

  passed
some seq:
- 1
- "'with map'":
    test: passed
    '"quoted"': kek
- 3"#;

    let docs = parse_yaml(config).expect("parsed");

    assert_eq!(docs.len(), 1);
    let doc = docs.get(0).expect("docs");

    let mut output = String::new();

    let () = doc.serialize(&mut output).expect("serialization error");

    assert_eq!(output.as_str(), config);
}
