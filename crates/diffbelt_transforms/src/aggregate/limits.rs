use crate::aggregate::AggregateTransform;

#[derive(Debug)]
pub struct Limits {
    pub pending_diffs_count: usize,
    pub pending_reduces_count: usize,
    pub has_more_diffs: bool,
    pub pending_eval_map_bytes: usize,
    pub target_data_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            pending_eval_map_bytes: 0,
            target_data_bytes: 0,
            has_more_diffs: true,
            pending_diffs_count: 0,
            pending_reduces_count: 0,
        }
    }
}

impl AggregateTransform {
    pub fn can_request_diff(max_limits: &Limits, current_limits: &Limits) -> bool {
        current_limits.pending_eval_map_bytes < max_limits.pending_eval_map_bytes
            && current_limits.target_data_bytes <= max_limits.target_data_bytes
    }

    pub fn can_eval_apply(max_limits: &Limits, current_limits: &Limits) -> bool {
        current_limits.target_data_bytes > max_limits.target_data_bytes
            || (!current_limits.has_more_diffs
                && current_limits.pending_diffs_count == 0
                && current_limits.pending_reduces_count == 0)
    }
}
