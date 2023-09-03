use crate::code::Code;
use serde::Deserialize;
use crate::transforms::TranformTargetKey;

#[derive(Debug, Deserialize)]
pub struct UniqueCount {
    pub target_key: TranformTargetKey,
    pub intermediate: Code,
    pub empty_target: Code,
    pub apply: Code,
}