use std::io::{stdout, Write};

#[allow(dead_code)]
pub fn debug_print(s: &str) {
    let mut out = stdout();
    let s_bytes = s.as_bytes();
    let mut bytes = vec![0u8; s_bytes.len() + 1].into_boxed_slice();
    for i in 0..(s_bytes.len()) {
        bytes[i] = s_bytes[i];
    }
    bytes[s_bytes.len()] = 10;
    out.write(&bytes).unwrap();
    out.flush().unwrap();
}
