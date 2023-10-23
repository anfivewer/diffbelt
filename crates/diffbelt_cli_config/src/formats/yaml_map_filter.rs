use diffbelt_protos::protos::transform::map_filter::{
    MapFilterInput, MapFilterInputArgs, MapFilterInputBuilder, MapFilterMultiInput,
    MapFilterMultiInputArgs, MapFilterMultiInputBuilder,
};
use diffbelt_protos::{OwnedSerialized, Serializer};
use thiserror::Error;

use diffbelt_yaml::YamlNode;

use crate::config_tests::value::{parse_scalar, ScalarParseError};
use crate::CollectionValueFormat;

#[derive(Error, Debug)]
pub enum YamlTestVarsError {
    #[error(transparent)]
    ScalarParse(#[from] ScalarParseError),
    #[error("{0}")]
    Unspecified(String),
}

impl CollectionValueFormat {
    pub fn yaml_test_vars_to_map_filter_input(
        &self,
        node: &YamlNode,
    ) -> Result<OwnedSerialized, YamlTestVarsError> {
        let mut serializer = Serializer::new();

        match self {
            CollectionValueFormat::Bytes | CollectionValueFormat::Utf8 => {
                let map = node.as_mapping().ok_or_else(|| {
                    YamlTestVarsError::Unspecified("vars should be a mapping".to_string())
                })?;

                let mut source_key_offset = None;
                let mut source_old_value_offset = None;
                let mut source_new_value_offset = None;

                for (key, value) in map {
                    let key = key.as_str().ok_or_else(|| {
                        YamlTestVarsError::Unspecified("vars key should be string".to_string())
                    })?;
                    let value = value.as_str().ok_or_else(|| {
                        YamlTestVarsError::Unspecified("vars value should be a string".to_string())
                    })?;

                    match key {
                        "source_key" => {
                            source_key_offset = Some(serializer.create_vector(value.as_bytes()));
                        }
                        "source_old_value" => {
                            if let Some(s) = parse_scalar(value)?.as_str() {
                                source_old_value_offset =
                                    Some(serializer.create_vector(s.as_bytes()));
                            }
                        }
                        "source_new_value" => {
                            if let Some(s) = parse_scalar(value)?.as_str() {
                                source_new_value_offset =
                                    Some(serializer.create_vector(s.as_bytes()));
                            }
                        }
                        _ => {
                            return Err(YamlTestVarsError::Unspecified(format!(
                                "unknown vars key: {key}"
                            )));
                        }
                    }
                }

                let input = MapFilterInput::create(
                    serializer.buffer_builder(),
                    &MapFilterInputArgs {
                        source_key: source_key_offset,
                        source_old_value: source_old_value_offset,
                        source_new_value: source_new_value_offset,
                    },
                );

                let offset = serializer.create_vector(&[input]);

                let offset = MapFilterMultiInput::create(
                    serializer.buffer_builder(),
                    &MapFilterMultiInputArgs {
                        items: Some(offset),
                    },
                );

                return Ok(serializer.finish(offset).into_owned());
            }
            CollectionValueFormat::Json => {
                todo!()
            }
        }
    }
}
