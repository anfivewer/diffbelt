#[inline(always)]
#[cfg(target_pointer_width = "64")]
pub fn usize_to_u64(value: usize) -> u64 {
    value as u64
}

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn unchecked_usize_to_u32(value: usize) -> u32 {
    value as u32
}

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn unchecked_usize_to_i32(value: usize) -> i32 {
    value as i32
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
pub fn u32_to_u64(value: u32) -> u64 {
    value as u64
}

#[inline(always)]
pub fn ptr_to_usize<T>(value: *const T) -> usize {
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
pub fn try_positive_i32_to_u32(value: i32) -> Option<u32> {
    if value < 0 {
        return None;
    }

    Some(value as u32)
}

#[inline(always)]
pub fn unchecked_i32_to_u32(value: i32) -> u32 {
    value as u32
}

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn try_positive_i32_to_usize(value: i32) -> Option<usize> {
    if value < 0 {
        return None;
    }

    Some(value as usize)
}

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn try_usize_to_u32(value: usize) -> Option<u32> {
    if value >= u32::MAX as usize {
        return None;
    }

    Some(value as u32)
}

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn try_usize_to_i32(value: usize) -> Option<i32> {
    if value >= i32::MAX as usize {
        return None;
    }

    Some(value as i32)
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

#[inline(always)]
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
pub fn checked_usize_to_i32(value: usize) -> i32 {
    assert!(value <= i32::MAX as usize, "{value} > i32::MAX",);
    value as i32
}

#[inline(always)]
pub fn checked_usize_to_isize(value: usize) -> isize {
    assert!(value <= isize::MAX as usize, "{value} > isize::MAX",);
    value as isize
}

#[inline(always)]
pub fn unchecked_isize_to_usize(value: isize) -> usize {
    value as usize
}

pub fn div_u64_usize_to_f64(a: u64, b: usize) -> f64 {
    a as f64 / b as f64
}
