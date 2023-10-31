use crate::config_tests::error::YamlTestVarsError;
use crate::config_tests::value::parse_scalar;
use diffbelt_yaml::YamlNode;

pub fn yaml_test_output_to_map_filter_expected_output(
    node: &YamlNode,
) -> Result<Vec<(&str, Option<&str>)>, YamlTestVarsError> {
    let seq = node.as_sequence().ok_or_else(|| {
        YamlTestVarsError::Unspecified("yaml test output should be a sequence".to_string())
    })?;

    let mut result = Vec::with_capacity(seq.items.len());

    for item in seq {
        let map = item.as_mapping().ok_or_else(|| {
            YamlTestVarsError::Unspecified(
                "yaml test output should be sequence of mappings".to_string(),
            )
        })?;

        let mut record_key = None;
        let mut record_value = None;

        for (key, value) in map {
            let (Some(key), Some(value)) = (key.as_str(), value.as_str()) else {
                return Err(YamlTestVarsError::Unspecified(
                    "yaml test output should be sequence of mappings of key-values which are should be a strings".to_string(),
                ));
            };

            let value = parse_scalar(value)?.as_str();

            match key {
                "key" => {
                    record_key = value;
                }
                "value" => {
                    record_value = value;
                }
                _ => {
                    return Err(YamlTestVarsError::Unspecified(format!(
                        "Unknown yaml test output key: {key}, it can be only \"key\" or \"value\""
                    )));
                }
            }
        }

        let Some(record_key) = record_key else {
            return Err(YamlTestVarsError::Unspecified(
                "yaml test output records should have key field".to_string(),
            ));
        };

        result.push((record_key, record_value));
    }

    Ok(result)
}
