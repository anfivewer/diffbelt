use serde::de::Error;
use serde::{Deserialize, Deserializer};

use crate::errors::{ConfigPositionMark, WithMark};

#[derive(Debug)]
pub struct WasmMethodDef {
    pub mark: ConfigPositionMark,
    pub module_name: String,
    pub method_name: String,
}

impl<'de> Deserialize<'de> for WasmMethodDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: WithMark<&str> = Deserialize::deserialize(deserializer)?;
        let WithMark { value, mark } = s;

        let Some((module_name, method_name)) = value.split_once('.') else {
            return Err(D::Error::custom("MapFilterWasm: should contain dot (.)"));
        };

        Ok(Self {
            mark,
            module_name: module_name.to_string(),
            method_name: method_name.to_string(),
        })
    }
}
