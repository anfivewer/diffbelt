use crate::serde::error::{ExpectError, YamlDecodingError};
use crate::{YamlMark, YamlNode};
use serde::de::value::{BorrowedStrDeserializer, U64Deserializer};
use serde::de::{DeserializeSeed, Error, MapAccess, Visitor};
use serde::Deserializer;
use std::fmt::Formatter;
use std::marker::PhantomData;

pub const WITH_MARK_NAME: &str = "__diffbelt_yaml_with_mark__private_struct";
pub const WITH_MARK_INDEX: &str = "__diffbelt_yaml_with_mark__private_index";
pub const WITH_MARK_LINE: &str = "__diffbelt_yaml_with_mark__private_line";
pub const WITH_MARK_COLUMN: &str = "__diffbelt_yaml_with_mark__private_column";
pub const WITH_MARK_VALUE: &str = "__diffbelt_yaml_with_mark__private_value";

pub trait Mark: Sized {
    fn new(index: u64, line: u64, column: u64) -> Self;
}

impl Mark for YamlMark {
    fn new(index: u64, line: u64, column: u64) -> Self {
        Self {
            index,
            line,
            column,
        }
    }
}

#[derive(Debug)]
pub struct WithMark<T, M> {
    pub value: T,
    pub mark: M,
}

struct WithMarkVisitor<T, M> {
    data: PhantomData<T>,
    mark: PhantomData<M>,
}

impl<'de, T: serde::de::Deserialize<'de>, M: Mark> Visitor<'de> for WithMarkVisitor<T, M> {
    type Value = WithMark<T, M>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("expected WithMark")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let (key, index) = map
            .next_entry::<&str, u64>()?
            .ok_or_else(|| A::Error::custom("WithMarkVisitor: no index entry"))?;
        if key != WITH_MARK_INDEX {
            return Err(A::Error::custom("WithMarkVisitor: key order missmatch"));
        }

        let (key, line) = map
            .next_entry::<&str, u64>()?
            .ok_or_else(|| A::Error::custom("WithMarkVisitor: no line entry"))?;
        if key != WITH_MARK_LINE {
            return Err(A::Error::custom("WithMarkVisitor: key order missmatch"));
        }

        let (key, column) = map
            .next_entry::<&str, u64>()?
            .ok_or_else(|| A::Error::custom("WithMarkVisitor: no column entry"))?;
        if key != WITH_MARK_COLUMN {
            return Err(A::Error::custom("WithMarkVisitor: key order missmatch"));
        }

        let (key, value) = map
            .next_entry::<&str, T>()?
            .ok_or_else(|| A::Error::custom("WithMarkVisitor: no index entry"))?;
        if key != WITH_MARK_VALUE {
            return Err(A::Error::custom("WithMarkVisitor: key order missmatch"));
        }

        Ok(WithMark {
            value,
            mark: M::new(index, line, column),
        })
    }
}

impl<'de, T: serde::de::Deserialize<'de>, M: Mark> serde::de::Deserialize<'de> for WithMark<T, M> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_struct(
            WITH_MARK_NAME,
            &[
                WITH_MARK_INDEX,
                WITH_MARK_LINE,
                WITH_MARK_COLUMN,
                WITH_MARK_VALUE,
            ],
            WithMarkVisitor {
                data: PhantomData::<T>::default(),
                mark: PhantomData::<M>::default(),
            },
        )
    }
}

pub struct WithMarkDe<'de> {
    pub node: &'de YamlNode,
    pub fields: &'de [&'de str],
    pub key_index: usize,
    pub value_index: usize,
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
                let de = super::Deserializer { input: self.node };

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
