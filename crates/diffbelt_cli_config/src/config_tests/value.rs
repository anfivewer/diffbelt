use std::ops::Deref;

use thiserror::Error;

use diffbelt_yaml::YamlNode;

#[derive(Debug)]
pub enum YamlValueConstructionError {
    Unspecified(String),
}

pub enum Scalar<'a> {
    String(&'a str),
    None,
}

impl<'a> Scalar<'a> {
    pub fn as_str(&'_ self) -> Option<&'a str> {
        match self {
            Scalar::String(s) => Some(s),
            Scalar::None => None,
        }
    }
}

#[derive(Error, Debug)]
pub enum ScalarParseError {
    #[error("{0}")]
    Unspecified(String),
}

pub fn parse_scalar(scalar: &YamlNode) -> Result<Scalar, ScalarParseError> {
    if let Some(tag) = &scalar.tag {
        if tag.deref() == "!none" {
            return Ok(Scalar::None);
        }
    }

    let Some(s) = scalar.as_str() else {
        return Err(ScalarParseError::Unspecified(format!(
            "Scalar not a string {:?}",
            scalar.start_mark,
        )));
    };

    let s = s.trim();

    Ok(Scalar::String(s))
}
