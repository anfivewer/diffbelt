use crate::errors::ConfigParsingError;
use crate::{FromYaml, YamlParsingState};
use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt::Formatter;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct VarsInstruction {
    #[serde(deserialize_with = "deserialize_vars")]
    pub vars: Vec<Var>,
}

struct VarsList;
struct VarsInstructionVisitor;

impl<'de> Visitor<'de> for VarsInstructionVisitor {
    type Value = Vec<Var>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("VarsInstructionVisitor: expected vars")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut vars = Vec::new();

        while let Some((name, value)) = map.next_entry::<String, VarProcessing>()? {
            vars.push(Var { name, value })
        }

        Ok(vars)
    }
}

fn deserialize_vars<'de, D>(deserializer: D) -> Result<Vec<Var>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(VarsInstructionVisitor)
}

#[derive(Debug, Eq, PartialEq)]
pub struct Var {
    pub name: String,
    pub value: VarProcessing,
}

#[derive(Debug, Eq, PartialEq)]
pub enum VarProcessing {
    ByString(String),
    DateFromUnixMs(DateFromUnixMsProcessing),
    Unknown(YamlNode),
}

impl<'de> Deserialize<'de> for VarProcessing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Deserialize::deserialize(deserializer)?;

        if let Ok(value) = decode_yaml(raw) {
            return Ok(VarProcessing::ByString(value));
        }
        if let Ok(value) = decode_yaml(raw) {
            return Ok(VarProcessing::DateFromUnixMs(value));
        }

        Ok(VarProcessing::Unknown(raw.clone()))
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct DateFromUnixMsProcessing {
    date_from_unix_ms: String,
}

impl VarProcessing {
    pub fn from_yaml(
        _state: &mut YamlParsingState,
        yaml: &YamlNode,
    ) -> Result<Self, ConfigParsingError> {
        Ok(decode_yaml(yaml)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::code::vars::{DateFromUnixMsProcessing, VarProcessing, VarsInstruction};
    use diffbelt_yaml::{decode_yaml, parse_yaml};

    #[test]
    fn parsing_test() {
        let input = r#"
vars:
  date:
    date_from_unix_ms: source.timestampMilliseconds
  key: some_string
"#;

        let input = &parse_yaml(input).expect("parsing")[0];
        let value: VarsInstruction = decode_yaml(input).expect("decode");

        assert_eq!(value.vars.len(), 2);
        assert_eq!(value.vars[0].name.as_str(), "date");
        assert_eq!(
            value.vars[0].value,
            VarProcessing::DateFromUnixMs(DateFromUnixMsProcessing {
                date_from_unix_ms: "source.timestampMilliseconds".to_string()
            })
        );
        assert_eq!(value.vars[1].name.as_str(), "key");
        assert_eq!(
            value.vars[1].value,
            VarProcessing::ByString("some_string".to_string())
        );
    }
}
