use alloc::rc::Rc;
use core::cmp::Ordering;
use core::ops::Range;
use core::str::{from_utf8, from_utf8_unchecked};
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Clone, Eq)]
pub struct ParsedIntermediateKey {
    pub key: Rc<[u8]>,
    value_slice: Range<usize>,
}

impl ParsedIntermediateKey {
    pub fn value(&self) -> &str {
        // SAFETY: checked on creation, `key` is immutable
        unsafe { from_utf8_unchecked(&self.key[self.value_slice.clone()]) }
    }
}

impl PartialEq for ParsedIntermediateKey {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}

impl PartialOrd for ParsedIntermediateKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key.partial_cmp(&other.key)
    }
}

impl Ord for ParsedIntermediateKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

lazy_static! {
    static ref PERCENTILES_REGEX: Regex =
        Regex::new(r#"^[^ ]+ ([^ ]+) "#).expect("Cannot build PERCENTILES_REGEX");
}

pub fn parse_and_store_intermediate_key(key: &[u8]) -> ParsedIntermediateKey {
    let key_str = from_utf8(key).expect("not utf8");
    let Some(captures) = PERCENTILES_REGEX.captures(key_str) else {
        panic!("does not match: {key_str}");
    };
    let value = captures.get(1).expect("captures").as_str();

    let key_offset = key.as_ptr() as usize;
    let value_offset = value.as_ptr() as usize;
    assert!(value_offset >= key_offset);

    let start = value_offset - key_offset;
    let bytes_len = value.len();

    ParsedIntermediateKey {
        key: Rc::from(key_str.as_bytes()),
        value_slice: start..(start + bytes_len),
    }
}
