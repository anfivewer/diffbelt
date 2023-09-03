use crate::code::Code;
use crate::transforms::TranformTargetKey;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UniqueCount {
    pub target_key: TranformTargetKey,
    pub intermediate: Code,
    pub empty_target: Code,
    pub apply: Code,
}
