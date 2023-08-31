use std::ops::Deref;
use crate::{YamlMapping, YamlNode, YamlNodeValue, YamlSequence};

impl YamlNode {
    pub fn as_string(&self) -> Option<&str> {
        let YamlNodeValue::Scalar(scalar) = &self.value else {
            return None;
        };

        Some(scalar.value.deref())
    }

    pub fn as_sequence(&self) -> Option<&YamlSequence> {
        let YamlNodeValue::Sequence(sequence) = &self.value else {
            return None;
        };

        Some(sequence)
    }

    pub fn as_mapping(&self) -> Option<&YamlMapping> {
        let YamlNodeValue::Mapping(mapping) = &self.value else {
            return None;
        };

        Some(mapping)
    }
}
