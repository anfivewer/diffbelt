use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use diffbelt_yaml::{decode_yaml, parse_yaml, YamlNode, YamlParsingError};

use crate::config_tests::TestSuite;
use crate::errors::{ConfigParsingError, ExpectedError};
use crate::formats::collection_human_readable_config::CollectionHumanReadableConfig;
use crate::transforms::Transform;
use crate::util::expect::{expect_bool, expect_map, expect_seq, expect_str};
use crate::wasm::{NewWasmInstanceOptions, Wasm, WasmError, WasmModuleInstance};

pub mod config_tests;
pub mod errors;
pub mod formats;
pub mod transforms;
pub mod util;
pub mod wasm;

#[cfg(not(target_endian = "little"))]
compile_error!("Only LE targets are supported because we are copying data to WASM");

#[derive(Debug)]
pub struct CliConfig {
    self_path: Rc<str>,

    collections: Vec<Collection>,
    transforms: Vec<Transform>,
    wasm: HashMap<Rc<str>, Wasm>,
    tests: HashMap<Rc<str>, TestSuite>,
}

#[derive(Debug)]
pub enum ParseConfigError {
    YamlParsing(YamlParsingError),
    ConfigParsing(ConfigParsingError),
}

impl CliConfig {
    fn from_yaml(self_path: Rc<str>, yaml: &YamlNode) -> Result<Self, ConfigParsingError> {
        let root = expect_map(yaml)?;

        let mut collections = Vec::new();
        let mut transforms = None;
        let mut wasm = HashMap::new();
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
                "wasm" => {
                    let wasm_node = expect_seq(&value)?;

                    for node in wasm_node.items.deref() {
                        let wasm_item: Wasm = decode_yaml(node)?;
                        wasm.insert(wasm_item.name.clone(), wasm_item);
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
            self_path,
            collections,
            transforms: transforms.unwrap_or_else(|| Vec::new()),
            wasm,
            tests: tests.unwrap_or_else(|| HashMap::new()),
        })
    }

    pub fn from_str(self_path: Rc<str>, config_str: &str) -> Result<Self, ParseConfigError> {
        let docs = parse_yaml(config_str).map_err(ParseConfigError::YamlParsing)?;
        let doc = &docs[0];
        let config =
            CliConfig::from_yaml(self_path, doc).map_err(ParseConfigError::ConfigParsing)?;

        Ok(config)
    }

    pub fn transform_names(&self) -> impl Iterator<Item = &str> {
        self.transforms
            .iter()
            .map(|transform| transform.name.as_ref())
            .filter_map(|name| name)
            .map(|name| name.deref())
    }

    pub fn wasm_module_def_by_name(&self, name: &str) -> Option<&Wasm> {
        self.wasm.get(name)
    }

    pub async fn new_wasm_instance(&self, wasm: &Wasm) -> Result<WasmModuleInstance, WasmError> {
        wasm.new_wasm_instance(NewWasmInstanceOptions {
            config_path: self.self_path.deref(),
        }).await
    }

    pub fn collection_by_name(&self, collection_name: &str) -> Option<&Collection> {
        self.collections
            .iter()
            .find(|collection| collection.name.deref() == collection_name)
    }

    pub fn transform_by_name(&self, required_name: &str) -> Option<&Transform> {
        self.transforms.iter().find(|transform| {
            transform
                .name
                .as_ref()
                .map(|name| name.deref() == required_name)
                .unwrap_or(false)
        })
    }
}

#[derive(Debug)]
pub struct Collection {
    pub name: Rc<str>,
    pub manual: bool,
    pub human_readable: Option<CollectionHumanReadableConfig>,
}

impl Collection {
    fn from_yaml(yaml: &YamlNode) -> Result<Self, ConfigParsingError> {
        let map = expect_map(yaml)?;

        let mut name = None;
        let mut manual = true;
        let mut human_readable = None;

        for (key_node, value) in &map.items {
            let key = expect_str(&key_node)?;

            match key {
                "name" => {
                    let value = expect_str(&value)?;
                    name = Some(Rc::from(value));
                }
                "manual" => {
                    manual = expect_bool(&value)?;
                }
                "human_readable" => {
                    human_readable = Some(decode_yaml(value)?);
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
            human_readable,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use diffbelt_yaml::parse_yaml;

    use crate::CliConfig;

    #[test]
    fn read_config() {
        let config_str = include_str!("../../../examples/cli-config.yaml");

        let docs = parse_yaml(config_str).expect("parsing");

        assert_eq!(docs.len(), 1);

        let doc = &docs[0];

        let config = CliConfig::from_yaml(Rc::from("."), doc).expect("reading");

        let _ = config;
    }
}
