use crate::errors::{
    ConfigParsingError, ExpectedError,
};
use yaml_peg::{Map, Node, Seq};

pub fn expect_map<Repr: yaml_peg::repr::Repr>(
    yaml: &Node<Repr>,
) -> Result<Map<Repr>, ConfigParsingError> {
    yaml.as_map().map_err(|position| {
        ConfigParsingError::ExpectedMap(ExpectedError {
            message: yaml_peg::dump(&[yaml.clone()], &[]),
            position,
        })
    })
}

pub fn expect_seq<Repr: yaml_peg::repr::Repr>(
    yaml: &Node<Repr>,
) -> Result<Seq<Repr>, ConfigParsingError> {
    yaml.as_seq().map_err(|position| {
        ConfigParsingError::ExpectedSeq(ExpectedError {
            message: yaml_peg::dump(&[yaml.clone()], &[]),
            position,
        })
    })
}

pub fn expect_str<Repr: yaml_peg::repr::Repr>(
    yaml: &Node<Repr>,
) -> Result<&str, ConfigParsingError> {
    yaml.as_str().map_err(|position| {
        let key = yaml_peg::dump(&[yaml.clone()], &[]);

        ConfigParsingError::ExpectedString(ExpectedError {
            message: key,
            position,
        })
    })
}

pub fn expect_bool<Repr: yaml_peg::repr::Repr>(
    yaml: &Node<Repr>,
) -> Result<bool, ConfigParsingError> {
    let value = yaml.as_str().map_err(|position| {
        let message = yaml_peg::dump(&[yaml.clone()], &[]);

        ConfigParsingError::ExpectedBool(ExpectedError { message, position })
    })?;

    let result = match value {
        "yes" => true,
        "no" => false,
        other => {
            return Err(ConfigParsingError::ExpectedBool(ExpectedError {
                message: other.to_string(),
                position: yaml.pos(),
            }));
        }
    };

    Ok(result)
}
