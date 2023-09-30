pub mod code;
pub mod errors;
pub mod interpreter;
pub mod config_tests;
pub mod transforms;
pub mod util;

use crate::code::Code;
use crate::errors::{ConfigParsingError, ExpectedError};
use crate::config_tests::TestSuite;
use crate::transforms::Transform;
use crate::util::expect::{expect_bool, expect_map, expect_seq, expect_str};
use diffbelt_yaml::{decode_yaml, YamlNode};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct CliConfig {
    collections: Vec<Collection>,
    transforms: Vec<Transform>,
    functions: HashMap<String, Code>,
    tests: HashMap<Rc<str>, TestSuite>,
}

impl CliConfig {
    fn from_yaml(yaml: &YamlNode) -> Result<Self, ConfigParsingError> {
        let root = expect_map(yaml)?;

        let mut collections = Vec::new();
        let mut functions = HashMap::new();
        let mut transforms = None;
        let mut tests = None;

        for (key_node, value) in &root.items {
            let key = expect_str(&key_node)?;

            match key {
                "collections" => {
                    let collections_node = expect_seq(&value)?;

                    for node in collections_node {
                        let collection = Collection::from_yaml(&node)?;
                        collections.push(collection);
                    }
                }
                "transforms" => {
                    transforms = Some(decode_yaml(value)?);
                }
                "functions" => {
                    let functions_node = expect_map(&value)?;

                    for (name, code) in &functions_node.items {
                        let name = expect_str(name)?;
                        let code = decode_yaml(code)?;

                        functions.insert(name.to_string(), code);
                    }
                }
                "tests" => {
                    let parsed_tests = decode_yaml(value)?;
                    tests = Some(parsed_tests);
                }
                other => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: other.to_string(),
                        position: Some((&key_node.start_mark).into()),
                    }));
                }
            }
        }

        Ok(Self {
            collections,
            functions,
            transforms: transforms.unwrap_or_else(|| Vec::new()),
            tests: tests.unwrap_or_else(|| HashMap::new()),
        })
    }
}

#[derive(Debug)]
pub struct Collection {
    name: String,
    manual: bool,
    format: CollectionValueFormat,
}

#[derive(Debug)]
pub enum CollectionValueFormat {
    Bytes,
    Utf8,
    Json,
}

impl Collection {
    fn from_yaml(yaml: &YamlNode) -> Result<Self, ConfigParsingError> {
        let map = expect_map(yaml)?;

        let mut name = None;
        let mut manual = true;
        let mut format = CollectionValueFormat::Bytes;

        for (key_node, value) in &map.items {
            let key = expect_str(&key_node)?;

            match key {
                "name" => {
                    let value = expect_str(&value)?;
                    name = Some(value.to_string());
                }
                "manual" => {
                    manual = expect_bool(&value)?;
                }
                "format" => {
                    let format_str = expect_str(&value)?;

                    format = match format_str {
                        "bytes" => CollectionValueFormat::Bytes,
                        "utf8" => CollectionValueFormat::Utf8,
                        "json" => CollectionValueFormat::Json,
                        other => {
                            return Err(ConfigParsingError::Custom(ExpectedError {
                                message: format!("unknown format: \"{}\"", other),
                                position: Some((&value.start_mark).into()),
                            }));
                        }
                    }
                }
                other => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: other.to_string(),
                        position: Some((&key_node.start_mark).into()),
                    }));
                }
            }
        }

        let name = name.ok_or_else(|| {
            ConfigParsingError::Custom(ExpectedError {
                message: "collection should have name".to_string(),
                position: Some((&yaml.start_mark).into()),
            })
        })?;

        Ok(Self {
            name,
            manual,
            format,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::CliConfig;
    use diffbelt_yaml::parse_yaml;

    #[test]
    fn read_config() {
        let config_str = include_str!("../../../examples/cli-config.yaml");

        let docs = parse_yaml(config_str).expect("parsing");

        assert_eq!(docs.len(), 1);

        let doc = &docs[0];

        let config = CliConfig::from_yaml(doc).expect("reading");

        let _ = config;
    }
}
