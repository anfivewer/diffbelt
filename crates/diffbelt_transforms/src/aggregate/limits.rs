use crate::aggregate::AggregateTransform;

#[derive(Debug)]
pub struct Limits {
    pub pending_diffs_count: usize,
    pub pending_reduces_count: usize,
    pub pending_merges_count: usize,
    pub pending_applies_count: usize,
    pub pending_puts_count: usize,
    pub has_more_diffs: bool,
    pub pending_eval_map_bytes: u64,
    pub target_data_bytes: u64,
    pub eval_apply_threshold: u64,
    pub applying_bytes: u64,
    pub pending_applying_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            pending_diffs_count: 0,
            pending_reduces_count: 0,
            pending_merges_count: 0,
            pending_applies_count: 0,
            pending_puts_count: 0,
            has_more_diffs: true,
            pending_eval_map_bytes: 0,
            target_data_bytes: 0,
            eval_apply_threshold: 0,
            applying_bytes: 0,
            pending_applying_bytes: 0,
        }
    }
}

impl AggregateTransform {
    #[inline(always)]
    pub fn can_request_diff(max_limits: &Limits, current_limits: &Limits) -> bool {
        current_limits.pending_eval_map_bytes < max_limits.pending_eval_map_bytes
            && (current_limits.target_data_bytes + current_limits.applying_bytes)
                <= max_limits.target_data_bytes
            || (current_limits.pending_diffs_count == 0
                && current_limits.pending_reduces_count == 0
                && current_limits.pending_merges_count == 0)
    }

    #[inline(always)]
    pub fn can_eval_apply(max_limits: &Limits, current_limits: &Limits) -> bool {
        Self::need_eval_apply(max_limits, current_limits)
            || (!current_limits.has_more_diffs
                && current_limits.pending_diffs_count == 0
                && current_limits.pending_reduces_count == 0)
    }

    #[inline(always)]
    pub fn need_eval_apply(max_limits: &Limits, current_limits: &Limits) -> bool {
        current_limits.target_data_bytes > max_limits.target_data_bytes
    }
}
