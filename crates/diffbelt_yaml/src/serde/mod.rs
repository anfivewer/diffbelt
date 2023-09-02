pub mod error;
#[cfg(test)]
mod tests;
mod with_mark;

use crate::serde::error::{ExpectError, YamlDecodingError};
use crate::serde::with_mark::{
    WITH_MARK_COLUMN, WITH_MARK_INDEX, WITH_MARK_LINE, WITH_MARK_NAME, WITH_MARK_VALUE,
};
use crate::{YamlMapping, YamlNode};
use serde::de::value::{BorrowedStrDeserializer, U64Deserializer};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
pub use with_mark::Mark;
pub use with_mark::WithMark;

pub fn decode_yaml<'de, T: Deserialize<'de>>(input: &'de YamlNode) -> Result<T, YamlDecodingError> {
    let de = Deserializer::from_yaml_node(&input);

    serde::de::Deserialize::deserialize(de)
}

pub struct Deserializer<'de> {
    input: &'de YamlNode,
}

impl<'de> Deserializer<'de> {
    pub fn from_yaml_node(input: &'de YamlNode) -> Self {
        Self { input }
    }
}

struct YamlMappingDe<'de> {
    mapping: &'de YamlMapping,
    iter_key: std::slice::Iter<'de, (YamlNode, YamlNode)>,
    iter_value: std::slice::Iter<'de, (YamlNode, YamlNode)>,
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

        let de = Deserializer::from_yaml_node(key);

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

        let de = Deserializer::from_yaml_node(value);

        seed.deserialize(de)
    }
}

struct WithMarkDe<'de> {
    node: &'de YamlNode,
    fields: &'de [&'de str],
    key_index: usize,
    value_index: usize,
}

impl<'de> MapAccess<'de> for WithMarkDe<'de> {
    type Error = YamlDecodingError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.key_index >= self.fields.len() {
            return Err(YamlDecodingError::Custom(ExpectError {
                message: "WithMarkDe: no more fields".to_string(),
                position: Some(self.node.start_mark.clone()),
            }));
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
                message: "WithMarkDe: no more fields".to_string(),
                position: Some(self.node.start_mark.clone()),
            }));
        }

        let field = self.fields[self.value_index];

        self.value_index += 1;

        let value = match field {
            WITH_MARK_INDEX => self.node.start_mark.index,
            WITH_MARK_LINE => self.node.start_mark.line,
            WITH_MARK_COLUMN => self.node.start_mark.column,
            WITH_MARK_VALUE => {
                let de = Deserializer { input: self.node };

                return seed.deserialize(de);
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

impl<'de> Deserializer<'de> {
    fn expect_str(&self) -> Result<&str, YamlDecodingError> {
        self.input.as_str().ok_or_else(|| {
            YamlDecodingError::Custom(ExpectError {
                message: "expected str".to_string(),
                position: Some(self.input.start_mark.clone()),
            })
        })
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for Deserializer<'de> {
    type Error = YamlDecodingError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.expect_str()?;

        match s {
            "y" => visitor.visit_bool(true),
            "yes" => visitor.visit_bool(true),
            "true" => visitor.visit_bool(true),
            "n" => visitor.visit_bool(false),
            "no" => visitor.visit_bool(false),
            "false" => visitor.visit_bool(false),
            _ => Err(YamlDecodingError::Custom(ExpectError {
                message: format!("expected bool, got \"{}\"", s),
                position: Some(self.input.start_mark.clone()),
            })),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.expect_str()?;

        let number = s.parse::<u64>().map_err(|_| {
            YamlDecodingError::Custom(ExpectError {
                message: format!("expected u64, got \"{}\"", s),
                position: Some(self.input.start_mark.clone()),
            })
        })?;

        visitor.visit_u64(number)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.expect_str()?;

        visitor.visit_str(s)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if name == WITH_MARK_NAME {
            return visitor.visit_map(WithMarkDe {
                node: self.input,
                fields,
                key_index: 0,
                value_index: 0,
            });
        }

        let mapping = self.input.as_mapping().ok_or_else(|| {
            YamlDecodingError::Custom(ExpectError {
                message: "expected map".to_string(),
                position: Some(self.input.start_mark.clone()),
            })
        })?;

        visitor.visit_map(YamlMappingDe {
            mapping,
            iter_key: mapping.items.iter(),
            iter_value: mapping.items.iter(),
        })
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.expect_str()?;

        visitor.visit_str(s)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(YamlDecodingError::Custom(ExpectError {
            message: "unexpected value".to_string(),
            position: Some(self.input.start_mark.clone()),
        }))
    }
}
