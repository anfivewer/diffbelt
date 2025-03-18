use diffbelt_cli_config::errors::RunTransformError;
use diffbelt_cli_config::CliConfig;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct MapFilterTransformInfo {
    pub wasm_module_name: String,
    pub wasm_method_name: String,
}

pub struct AggregateTransformInfo {
    pub wasm_module_name: String,
    pub map: String,
    pub initial_accumulator: String,
    pub reduce: String,
    pub merge_accumulators: Option<String>,
    pub apply: String,
}

pub enum TransformTypeInfo {
    MapFilter(MapFilterTransformInfo),
    Aggregate(AggregateTransformInfo),
}

pub struct TransformInfo {
    pub source_collection_name: String,
    pub intermediate_collection_name: Option<String>,
    pub target_collection_name: String,
    pub reader_name: Option<String>,
    pub transform: TransformTypeInfo,
}

pub struct CollectionInfo {
    pub is_manual: bool,
}

pub struct TransformConfig {
    pub wasm_root_path: PathBuf,
    pub transforms: HashMap<String, TransformInfo>,
    pub collections: HashMap<String, CollectionInfo>,
}

impl TransformConfig {
    pub fn new(config: &CliConfig) -> Result<Self, RunTransformError> {
        let mut collections = HashMap::with_capacity(config.collections.len());
        let mut transforms = HashMap::with_capacity(config.transforms.len());

        for collection in config.collections.iter() {
            collections.insert(
                collection.name.to_string(),
                CollectionInfo {
                    is_manual: collection.manual,
                },
            );
        }

        for transform in config.transforms.iter() {
            let Some(transform_name) = transform.name.as_ref() else {
                continue;
            };

            let transform_type = if let Some(wasm_def) = transform.map_filter.as_ref() {
                TransformTypeInfo::MapFilter(MapFilterTransformInfo {
                    wasm_module_name: wasm_def.module_name.clone(),
                    wasm_method_name: wasm_def.method_name.clone(),
                })
            } else if let Some(aggregate_def) = transform.aggregate.as_ref() {
                TransformTypeInfo::Aggregate(AggregateTransformInfo {
                    wasm_module_name: aggregate_def.wasm.clone(),
                    map: aggregate_def.map.clone(),
                    initial_accumulator: aggregate_def.initial_accumulator.clone(),
                    reduce: aggregate_def.reduce.clone(),
                    merge_accumulators: aggregate_def.merge_accumulators.clone(),
                    apply: aggregate_def.apply.clone(),
                })
            } else {
                return Err(RunTransformError::Message(
                    "Unsupported transform type".to_string(),
                ));
            };

            transforms.insert(
                transform_name.to_string(),
                TransformInfo {
                    source_collection_name: transform.source.to_string(),
                    intermediate_collection_name: transform
                        .intermediate
                        .as_ref()
                        .map(|x| x.to_string()),
                    target_collection_name: transform.target.to_string(),
                    reader_name: transform.reader_name.as_ref().map(|x| x.to_string()),
                    transform: transform_type,
                },
            );
        }

        Ok(Self {
            wasm_root_path: PathBuf::from(config.self_path.as_ref()),
            transforms,
            collections,
        })
    }
}
