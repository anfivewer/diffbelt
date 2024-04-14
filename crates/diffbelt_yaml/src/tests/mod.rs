use crate::parse_yaml;

mod multiline;
mod serialization;

#[test]
fn parse_cli_config() {
    let config = r#"
anchored: &test
  value: 42
  list: !tagged
    - with_values: yes
    - and_lists: [1, 'test', "something", 42]
      tratata: wuts
with_anchor: *test
"#;

    let docs = parse_yaml(config).expect("parsed");

    println!("{:?}", docs);
}
