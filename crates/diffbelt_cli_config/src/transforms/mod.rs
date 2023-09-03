use crate::code::Code;
use crate::transforms::aggregate::Aggregate;
use crate::transforms::map_filter::MapFilterYaml;
use crate::transforms::percentiles::Percentiles;
use crate::transforms::unique_count::UniqueCount;
use diffbelt_yaml::{decode_yaml, YamlNode};
use serde::{Deserialize, Deserializer};

pub mod aggregate;
pub mod map_filter;
pub mod percentiles;
pub mod unique_count;

#[derive(Debug, Deserialize)]
pub struct Transform {
    pub from: String,
    pub intermediate: Option<TransformCollectionDef>,
    pub to: TransformCollectionDef,
    pub reader_name: Option<String>,
    pub map_filter: Option<MapFilterYaml>,
    pub aggregate: Option<Aggregate>,
    pub percentiles: Option<Percentiles>,
    pub unique_count: Option<UniqueCount>,
}

#[derive(Debug)]
pub enum TransformCollectionDef {
    Named(String),
    WithReader(CollectionWithReader),
    Unknown(YamlNode),
}

#[derive(Debug)]
pub enum CollectionDef {
    Named(String),
    WithFormat(CollectionWithFormat),
    Unknown(YamlNode),
}

#[derive(Debug, Deserialize)]
pub struct CollectionWithReader {
    pub collection: CollectionDef,
    pub reader_name: String,
}

#[derive(Debug, Deserialize)]
pub struct CollectionWithFormat {
    pub name: String,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct TranformTargetKey {
    pub source: Code,
    pub intermediate: Code,
}

impl<'de> Deserialize<'de> for TransformCollectionDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Deserialize::deserialize(deserializer)?;

        if let Ok(value) = decode_yaml(raw) {
            return Ok(TransformCollectionDef::Named(value));
        }
        if let Ok(value) = decode_yaml(raw) {
            return Ok(TransformCollectionDef::WithReader(value));
        }

        Ok(TransformCollectionDef::Unknown(raw.clone()))
    }
}

impl<'de> Deserialize<'de> for CollectionDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Deserialize::deserialize(deserializer)?;

        if let Ok(value) = decode_yaml(raw) {
            return Ok(CollectionDef::Named(value));
        }
        if let Ok(value) = decode_yaml(raw) {
            return Ok(CollectionDef::WithFormat(value));
        }

        Ok(CollectionDef::Unknown(raw.clone()))
    }
}
