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
