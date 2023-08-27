pub mod code;
pub mod errors;
pub mod transforms;
pub mod util;

use crate::errors::{ConfigParsingError, ExpectedError};
use crate::transforms::Transform;
use crate::util::expect::{expect_bool, expect_map, expect_seq, expect_str};
use yaml_peg::repr::Repr;
use yaml_peg::NodeRc;

pub struct YamlParsingState {
    //
}

impl YamlParsingState {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug)]
pub struct CliConfig {
    collections: Vec<Collection>,
    transforms: Vec<Transform>,
}

impl CliConfig {
    pub fn from_yaml(
        state: &mut YamlParsingState,
        yaml: &NodeRc,
    ) -> Result<Self, ConfigParsingError> {
        let root = expect_map(yaml)?;

        let mut collections = Vec::new();

        for (key_node, value) in root {
            let key = expect_str(&key_node)?;

            match key {
                "collections" => {
                    let collections_node = expect_seq(&value)?;

                    for node in collections_node {
                        let collection = Collection::from_yaml(state, &node)?;
                        collections.push(collection);
                    }
                }
                "transforms" => {}
                "functions" => {}
                other => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: other.to_string(),
                        position: key_node.pos(),
                    }));
                }
            }
        }

        Ok(Self {
            collections,
            transforms: Vec::new(),
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
    /*
       name: log-lines
       manual: no
       format: utf8
    */
    pub fn from_yaml(
        state: &mut YamlParsingState,
        yaml: &NodeRc,
    ) -> Result<Self, ConfigParsingError> {
        let map = expect_map(yaml)?;

        let mut name = None;
        let mut manual = true;
        let mut format = CollectionValueFormat::Bytes;

        for (key_node, value) in map {
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
                                position: value.pos(),
                            }));
                        }
                    }
                }
                other => {
                    return Err(ConfigParsingError::UnknownKey(ExpectedError {
                        message: other.to_string(),
                        position: key_node.pos(),
                    }));
                }
            }
        }

        let name = name.ok_or_else(|| {
            ConfigParsingError::Custom(ExpectedError {
                message: "collection should have name".to_string(),
                position: yaml.pos(),
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
    use crate::{CliConfig, YamlParsingState};
    use yaml_peg::repr::RcRepr;

    #[test]
    fn read_config() {
        let config_str = include_str!("../../../examples/cli-config.yaml");

        let docs = yaml_peg::parser::parse::<RcRepr>(config_str).expect("parsing");

        assert_eq!(docs.len(), 1);

        let doc = &docs[0];

        let mut state = YamlParsingState::new();
        let config = CliConfig::from_yaml(&mut state, doc).expect("reading");

        println!("{:?}", config);
    }

    #[test]
    fn config_dumps() {
        let config_str = include_str!("../../../examples/cli-config.yaml");

        let docs = yaml_peg::parser::parse::<RcRepr>(config_str).expect("parsing");
        let docs = docs.to_vec();

        yaml_peg::dump(&docs, &[]);
    }
}
