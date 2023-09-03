pub mod error;
mod mapping;
mod raw;
mod sequence;
#[cfg(test)]
mod tests;
mod with_mark;

use crate::serde::error::{ExpectError, YamlDecodingError};
use crate::serde::mapping::YamlMappingDe;
use crate::serde::raw::{YamlNodeDe, RAW_YAML_NODE};
use crate::serde::sequence::YamlSequenceDe;
use crate::serde::with_mark::{
    WithMarkDe, WITH_MARK_COLUMN, WITH_MARK_INDEX, WITH_MARK_LINE, WITH_MARK_NAME, WITH_MARK_VALUE,
};
use crate::{YamlNode, YamlNodeValue};
use serde::de::value::{BorrowedStrDeserializer, U64Deserializer};
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde::Deserialize;
use std::ops::Deref;
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
        println!("any {:?}", self.input);
        match &self.input.value {
            YamlNodeValue::Empty => visitor.visit_unit(),
            YamlNodeValue::Scalar(scalar) => visitor.visit_borrowed_str(scalar.value.deref()),
            YamlNodeValue::Sequence(sequence) => visitor.visit_seq(YamlSequenceDe {
                sequence,
                iter: sequence.items.iter(),
            }),
            YamlNodeValue::Mapping(mapping) => visitor.visit_map(YamlMappingDe {
                mapping,
                iter_key: mapping.items.iter(),
                iter_value: mapping.items.iter(),
            }),
        }
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

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.expect_str()?;

        let number = s.parse::<u32>().map_err(|_| {
            YamlDecodingError::Custom(ExpectError {
                message: format!("expected u64, got \"{}\"", s),
                position: Some(self.input.start_mark.clone()),
            })
        })?;

        visitor.visit_u32(number)
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

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let s = self.expect_str()?;

        visitor.visit_str(s)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("newtype {}", _name);
        self.deserialize_any(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let sequence = self.input.as_sequence().ok_or_else(|| {
            YamlDecodingError::Custom(ExpectError {
                message: "YamlDeserializer: expected seq".to_string(),
                position: Some(self.input.start_mark.clone()),
            })
        })?;

        visitor.visit_seq(YamlSequenceDe {
            sequence,
            iter: sequence.items.iter(),
        })
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
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

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        println!("deserialize struct {} {:?}", name, fields);

        if name == WITH_MARK_NAME {
            return visitor.visit_map(WithMarkDe {
                node: self.input,
                fields,
                key_index: 0,
                value_index: 0,
            });
        }

        if name == RAW_YAML_NODE {
            return visitor.visit_map(YamlNodeDe {
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
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
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
