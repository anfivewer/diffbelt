use alloc::string::String;
use core::fmt::{Formatter, Write};
use core::str::from_utf8;

const HEX_TABLE: [char; 16] = ['0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f'];

#[allow(dead_code)]
pub fn bytes_as_hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);

    for b in bytes {
        let higher = b / 16;
        let lower = b % 16;

        result.push(HEX_TABLE[higher as usize]);
        result.push(HEX_TABLE[lower as usize]);
    }

    result
}

pub fn fmt_bytes_as_hex(bytes: &[u8], f: &mut Formatter<'_>) -> core::fmt::Result {
    for b in bytes {
        let higher = b / 16;
        let lower = b % 16;

        f.write_char(HEX_TABLE[higher as usize])?;
        f.write_char(HEX_TABLE[lower as usize])?;
    }

    Ok(())
}

pub fn fmt_bytes_as_str_or_hex(bytes: &[u8], f: &mut Formatter<'_>) -> core::fmt::Result {
    if let Ok(s) = from_utf8(bytes) {
        f.write_str(s)?;
        return Ok(());
    }

    fmt_bytes_as_hex(bytes, f)?;

    Ok(())
}
