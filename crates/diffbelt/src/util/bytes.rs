pub fn read_u24(bytes: &[u8], offset: usize) -> u32 {
    ((bytes[offset] as u32) << 16) + ((bytes[offset + 1] as u32) << 8) + (bytes[offset + 2] as u32)
}

pub fn write_u24(bytes: &mut [u8], offset: usize, value: u32) -> () {
    let mut value = value;
    bytes[offset + 2] = (value & 0xff) as u8;
    value >>= 8;
    bytes[offset + 1] = (value & 0xff) as u8;
    value >>= 8;
    bytes[offset] = (value & 0xff) as u8;
}

pub const ONE_U32_BE: [u8; 4] = [0, 0, 0, 1];

pub fn to_u32_be_unchecked(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) << 24)
        + ((bytes[1] as u32) << 16)
        + ((bytes[2] as u32) << 8)
        + ((bytes[3] as u32) << 0)
}

pub fn from_u32_be(value: u32) -> [u8; 4] {
    [
        ((value >> 24) & 0xff) as u8,
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        ((value >> 0) & 0xff) as u8,
    ]
}

pub fn increment(bytes: &mut [u8]) {
    for i in (0..bytes.len()).rev() {
        if bytes[i] == 255 {
            bytes[i] = 0;
            continue;
        }

        bytes[i] += 1;
        break;
    }
}

pub fn decrement(bytes: &mut [u8]) {
    for i in (0..bytes.len()).rev() {
        if bytes[i] == 0 {
            bytes[i] = 255;
            continue;
        }

        bytes[i] -= 1;
        break;
    }
}

pub fn is_byte_array_equal<'a, 'b, A: Into<&'a [u8]>, B: Into<&'b [u8]>>(
    this: A,
    other: B,
) -> bool {
    let a: &[u8] = this.into();
    let b: &[u8] = other.into();
    a == b
}

pub fn is_byte_array_equal_opt<'a, 'b, A: Into<&'a [u8]>, B: Into<&'b [u8]>>(
    this: A,
    other: Option<B>,
) -> bool {
    match other {
        Some(other) => is_byte_array_equal(this, other),
        None => false,
    }
}

pub fn is_byte_array_equal_both_opt<'a, 'b, A: Into<&'a [u8]>, B: Into<&'b [u8]>>(
    this: Option<A>,
    other: Option<B>,
) -> bool {
    match this {
        Some(this) => is_byte_array_equal_opt(this, other),
        None => other.is_none(),
    }
}
