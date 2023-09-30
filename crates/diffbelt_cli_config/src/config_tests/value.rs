use crate::interpreter::value::Value;
use diffbelt_util::Wrap;
use diffbelt_yaml::{YamlNode, YamlNodeValue};
use regex::Regex;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug)]
pub enum YamlValueConstructionError {
    Unspecified(String),
}

lazy_static::lazy_static! {
    static ref U64: Regex = Regex::new("^\\d+$").unwrap();
    static ref STR: Regex = Regex::new("^('?)(.*)('?)$").unwrap();
}

pub fn construct_value_from_yaml(node: &YamlNode) -> Result<Value, YamlValueConstructionError> {
    match &node.value {
        YamlNodeValue::Empty => Err(YamlValueConstructionError::Unspecified(
            "construct_value_from_yaml: empty node".to_string(),
        )),
        YamlNodeValue::Scalar(scalar) => {
            let scalar = scalar.value.deref();
            if let Some(captures) = U64.captures(scalar) {
                let value_str = captures.get(0).unwrap().as_str();
                let value = value_str.parse::<u64>();

                return value.map(Value::U64).map_err(|_| {
                    YamlValueConstructionError::Unspecified(format!(
                        "Cannot parse \"{value_str}\" as u64"
                    ))
                });
            }

            if let Some(captures) = STR.captures(scalar) {
                let first_quote = captures.get(1).unwrap().as_str();
                let value = captures.get(2).unwrap().as_str();
                let last_quote = captures.get(3).unwrap().as_str();

                if first_quote != last_quote {
                    return Err(YamlValueConstructionError::Unspecified(format!(
                        "Value \"{scalar}\" quotes missmatch"
                    )));
                }

                return Ok(Value::String(Rc::from(value)));
            }

            Err(YamlValueConstructionError::Unspecified(format!(
                "Unknown \"{scalar}\" type"
            )))
        }
        YamlNodeValue::Sequence(seq) => {
            let mut result = Vec::new();

            for node in seq {
                let value = construct_value_from_yaml(node)?;
                result.push(value);
            }

            Ok(Value::List(Wrap::wrap(result)))
        }
        YamlNodeValue::Mapping(map) => {
            let mut result = HashMap::with_capacity(map.items.len());

            for (key, value) in map {
                let key = construct_value_from_yaml(key)?;
                let value = construct_value_from_yaml(value)?;

                let key = key.as_primitive_value().map_err(|_| {
                    YamlValueConstructionError::Unspecified(
                        "not primitive value in map key".to_string(),
                    )
                })?;

                result.insert(key, value);
            }

            Ok(Value::Map(Wrap::wrap(result)))
        }
    }
}
