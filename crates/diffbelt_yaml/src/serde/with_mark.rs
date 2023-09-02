use crate::YamlMark;
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
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
