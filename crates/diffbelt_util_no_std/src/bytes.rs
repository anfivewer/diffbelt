pub fn try_4_bytes_be_to_u32(bytes: &[u8]) -> Option<u32> {
    if bytes.len() != 4 {
        return None;
    }

    let result = unsafe {
        ((*bytes.get_unchecked(0) as u32) << 24)
            + ((*bytes.get_unchecked(1) as u32) << 16)
            + ((*bytes.get_unchecked(2) as u32) << 8)
            + ((*bytes.get_unchecked(3) as u32) << 0)
    };

    Some(result)
}
