use crate::aggregate::AggregateTransform;

#[derive(Debug)]
pub struct Limits {
    pub pending_eval_map_bytes: usize,
    pub target_data_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            pending_eval_map_bytes: 0,
            target_data_bytes: 0,
        }
    }
}

impl AggregateTransform {
    pub fn can_request_diff(max_limits: &Limits, current_limits: &Limits) -> bool {
        current_limits.pending_eval_map_bytes < max_limits.pending_eval_map_bytes
    }
}
