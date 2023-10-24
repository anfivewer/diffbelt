#[inline(always)]
pub fn try_positive_i64_to_u64(value: i64) -> Option<u64> {
    if value >= 0 {
        Some(value as u64)
    } else {
        None
    }
}
