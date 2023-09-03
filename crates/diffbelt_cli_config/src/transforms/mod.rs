use serde::Deserialize;
use crate::transforms::map_filter::MapFilterYaml;

pub mod map_filter;

#[derive(Debug, Deserialize)]
pub struct Transform {
    from: String,
    to: String,
    reader_name: Option<String>,
    map_filter: Option<MapFilterYaml>,
}
