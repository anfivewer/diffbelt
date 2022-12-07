pub fn read_u24(bytes: &[u8], offset: usize) -> u32 {
    ((bytes[offset] as u32) << 16) + ((bytes[offset + 1] as u32) << 8) + (bytes[offset + 2] as u32)
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

#[test]
fn some_test() {
    for n in (1..3).rev() {
        println!("kek {}", n);
    }
}
