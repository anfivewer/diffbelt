use crate::{parse_yaml, YamlMark};
use crate::serde::{Deserializer, WithMark};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SimpleStruct {
    a: usize,
    b: String,
    c: bool,
}

#[derive(Debug, Deserialize)]
struct StructWithMark {
    field: WithMark<String, YamlMark>,
}

#[test]
fn deserialize_test() {
    let input = r#"
a: 42
b: test
c: yes
"#;

    let input = &parse_yaml(input).expect("parsing")[0];

    let de = Deserializer::from_yaml_node(&input);

    let value: SimpleStruct = serde::de::Deserialize::deserialize(de).expect("decoding");

    let SimpleStruct { a, b, c } = value;

    assert_eq!(a, 42);
    assert_eq!(b.as_str(), "test");
    assert_eq!(c, true);
}

#[test]
fn deserialize_with_mark_test() {
    let input = r#"# Some comment
field: test
"#;

    let input = &parse_yaml(input).expect("parsing")[0];

    let de = Deserializer::from_yaml_node(&input);

    let value: StructWithMark = serde::de::Deserialize::deserialize(de).expect("decoding");

    let StructWithMark {
        field: WithMark { value, mark },
    } = value;

    assert_eq!(value.as_str(), "test");
    assert_eq!(mark.index, 22);
    assert_eq!(mark.line, 1);
    assert_eq!(mark.column, 7);
}
