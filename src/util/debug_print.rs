use std::io::{stdout, Write};

#[allow(dead_code)]
pub fn debug_print(s: &str) {
    let mut out = stdout();
    out.write(s.as_bytes()).unwrap();
    out.write("\n".as_bytes()).unwrap();
    out.flush().unwrap();
}
