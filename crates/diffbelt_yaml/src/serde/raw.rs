use crate::serde::error::{ExpectError, YamlDecodingError};
use crate::serde::static_trespass::{save_yaml_node, take_yaml_node};
use crate::{YamlNode, YamlNodeRc};
use serde::de::value::{BorrowedStrDeserializer, U64Deserializer};
use serde::de::{DeserializeSeed, Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt::Formatter;
use std::rc::Rc;

pub const RAW_YAML_NODE: &str = "__diffbelt_yaml_raw_yaml_node__private_struct";
pub const RAW_YAML_NODE_VALUE: &str = "__diffbelt_yaml_raw_yaml_node__private_value";

struct YamlNodeVisitor;

impl<'de> Visitor<'de> for YamlNodeVisitor {
    type Value = Rc<YamlNode>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("YamlNode")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let (key, trespass_counter) = map
            .next_entry::<&str, u64>()?
            .ok_or_else(|| A::Error::custom("YamlNodeVisitor: no value entry"))?;

        if key != RAW_YAML_NODE_VALUE {
            return Err(A::Error::custom("YamlNodeVisitor: key order missmatch"));
        }

        let node = take_yaml_node(trespass_counter).expect("YamlNodeVisitor: not received");

        Ok(node)
    }
}

impl<'de> serde::de::Deserialize<'de> for YamlNodeRc {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let node = deserializer.deserialize_struct(
            RAW_YAML_NODE,
            &[RAW_YAML_NODE_VALUE],
            YamlNodeVisitor,
        )?;

        Ok(YamlNodeRc(node))
    }
}

impl YamlNode {
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Rc<YamlNode>, D::Error> {
        let YamlNodeRc(node) = YamlNodeRc::deserialize(deserializer)?;
        Ok(node)
    }
}

pub struct YamlNodeDe<'de> {
    pub node: &'de Rc<YamlNode>,
    pub fields: &'de [&'de str],
    pub key_index: usize,
    pub value_index: usize,
}

impl<'de> MapAccess<'de> for YamlNodeDe<'de> {
    type Error = YamlDecodingError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.key_index >= self.fields.len() {
            return Ok(None);
        }

        let field = self.fields[self.key_index];

        self.key_index += 1;

        let de = BorrowedStrDeserializer::new(field);

        seed.deserialize(de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        if self.value_index >= self.fields.len() {
            return Err(YamlDecodingError::Custom(ExpectError {
                message: "YamlNodeDe: no more fields".to_string(),
                position: Some(self.node.start_mark.clone()),
            }));
        }

        let field = self.fields[self.value_index];

        self.value_index += 1;

        let value = match field {
            RAW_YAML_NODE_VALUE => {
                let node = self.node.clone();
                save_yaml_node(node)
            }
            _ => {
                return Err(YamlDecodingError::Custom(ExpectError {
                    message: "WithMarkDe: unsupported field".to_string(),
                    position: Some(self.node.start_mark.clone()),
                }));
            }
        };

        let de = U64Deserializer::new(value);

        seed.deserialize(de)
    }
}
