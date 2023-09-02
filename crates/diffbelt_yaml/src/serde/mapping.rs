use serde::de::{DeserializeSeed, MapAccess};
use crate::{YamlMapping, YamlNode};
use crate::serde::error::{ExpectError, YamlDecodingError};

pub struct YamlMappingDe<'de> {
    pub mapping: &'de YamlMapping,
    pub iter_key: std::slice::Iter<'de, (YamlNode, YamlNode)>,
    pub iter_value: std::slice::Iter<'de, (YamlNode, YamlNode)>,
}

impl<'de> MapAccess<'de> for YamlMappingDe<'de> {
    type Error = YamlDecodingError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where
            K: DeserializeSeed<'de>,
    {
        let Some((key, _)) = self.iter_key.next() else {
            return Ok(None);
        };

        let de = super::Deserializer::from_yaml_node(key);

        seed.deserialize(de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where
            V: DeserializeSeed<'de>,
    {
        let Some((_, value)) = self.iter_value.next() else {
            return Err(YamlDecodingError::Custom(ExpectError {
                message: "unexpected end".to_string(),
                position: None,
            }));
        };

        let de = super::Deserializer::from_yaml_node(value);

        seed.deserialize(de)
    }
}