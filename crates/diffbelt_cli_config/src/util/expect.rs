use crate::errors::{ConfigParsingError, ExpectedError};
use diffbelt_yaml::{YamlMapping, YamlNode, YamlSequence};

pub fn expect_map(yaml: &YamlNode) -> Result<&YamlMapping, ConfigParsingError> {
    yaml.as_mapping().ok_or_else(|| {
        ConfigParsingError::ExpectedMap(ExpectedError {
            message: "expected map".to_string(),
            position: Some((&yaml.start_mark).into()),
        })
    })
}

pub fn expect_seq(yaml: &YamlNode) -> Result<&YamlSequence, ConfigParsingError> {
    yaml.as_sequence().ok_or_else(|| {
        ConfigParsingError::ExpectedSeq(ExpectedError {
            message: "expected sequence".to_string(),
            position: Some((&yaml.start_mark).into()),
        })
    })
}

pub fn expect_str(yaml: &YamlNode) -> Result<&str, ConfigParsingError> {
    yaml.as_str().ok_or_else(|| {
        ConfigParsingError::ExpectedString(ExpectedError {
            message: "expected string".to_string(),
            position: Some((&yaml.start_mark).into()),
        })
    })
}

pub fn expect_bool(yaml: &YamlNode) -> Result<bool, ConfigParsingError> {
    let value = yaml.as_str().ok_or_else(|| {
        ConfigParsingError::ExpectedBool(ExpectedError {
            message: "expected bool".to_string(),
            position: Some((&yaml.start_mark).into()),
        })
    })?;

    let result = match value {
        "yes" => true,
        "no" => false,
        _ => {
            return Err(ConfigParsingError::ExpectedBool(ExpectedError {
                message: "expected bool".to_string(),
                position: Some((&yaml.start_mark).into()),
            }));
        }
    };

    Ok(result)
}
