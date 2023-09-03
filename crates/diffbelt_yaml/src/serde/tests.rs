use crate::serde::{Deserializer, WithMark};
use crate::{decode_yaml, parse_yaml, YamlMark, YamlNode};
use serde::de::Error;
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

#[derive(Debug, Deserialize)]
struct StructWithRaw {
    answer: u32,
    raw: YamlNode,
}

#[derive(Debug)]
enum EnumWithRaw {
    Variant(String),
    Raw(YamlNode),
}

impl<'de> Deserialize<'de> for EnumWithRaw {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw: &YamlNode = Deserialize::deserialize(deserializer)?;

        if let Ok(value) = decode_yaml(raw) {
            return Ok(EnumWithRaw::Variant(value));
        }
        if let Ok(value) = decode_yaml(raw) {
            return Ok(EnumWithRaw::Raw(value));
        }

        Err(D::Error::custom("bad news"))
    }
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

#[test]
fn deserialize_with_raw_test() {
    let input = r#"# Some comment
answer: 42
raw:
  - 1
  - 2
  - 3
"#;

    let input = &parse_yaml(input).expect("parsing")[0];

    let de = Deserializer::from_yaml_node(&input);

    let value: StructWithRaw = serde::de::Deserialize::deserialize(de).expect("decoding");

    let StructWithRaw { answer, raw } = value;

    assert_eq!(answer, 42);

    let values: Vec<&str> = raw
        .as_sequence()
        .expect("not a sequence")
        .items
        .iter()
        .map(|node| node.as_str().expect("not a str"))
        .collect();

    assert_eq!(values, vec!["1", "2", "3"]);
}

#[test]
fn deserialize_enum_with_raw_test() {
    let input = r#"# Some comment
answer: 42
"#;

    let input = &parse_yaml(input).expect("parsing")[0];

    let de = Deserializer::from_yaml_node(&input);

    let value: EnumWithRaw = serde::de::Deserialize::deserialize(de).expect("decoding");

    println!("value {:#?}", value);
}
