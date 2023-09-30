#[inline(always)]
#[cfg(target_pointer_width = "64")]
pub fn usize_to_u64(value: usize) -> u64 {
    value as u64
}

#[inline(always)]
#[cfg(target_pointer_width = "64")]
pub fn u64_to_usize(value: u64) -> usize {
    value as usize
}

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn u32_to_usize(value: u32) -> usize {
    value as usize
}

#[inline(always)]
pub fn u8_to_u64(value: u8) -> u64 {
    value as u64
}

#[inline(always)]
pub fn u8_to_usize(value: u8) -> usize {
    value as usize
}

#[inline(always)]
pub fn checked_positive_i64_to_u64(value: i64) -> u64 {
    assert!(
        value >= 0,
        "{} is less than 0 on attempt to cast i64 to u64",
        value
    );
    value as u64
}

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn checked_positive_i32_to_usize(value: i32) -> usize {
    assert!(
        value >= 0,
        "{} is less than 0 on attempt to cast i32 to usize",
        value
    );
    value as usize
}

#[inline(always)]
pub fn checked_positive_isize_to_usize(value: isize) -> usize {
    assert!(
        value >= 0,
        "{} is less than 0 on attempt to cast isize to usize",
        value
    );
    value as usize
}
