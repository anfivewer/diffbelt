use crate::util::bytes::read_u24;
use std::cmp::Ordering;

/*
    1 -- reserved byte
    3 -- size of user key
    1 -- size of generationId
    1 -- size of phantomId
*/
const MIN_KEY_SIZE: usize = 1 + 3 + 1 + 1;

pub fn record_key_compare_u24_sized(
    left: &[u8],
    right: &[u8],
    left_offset: usize,
    right_offset: usize,
) -> (Ordering, usize, usize) {
    let left_key_size = read_u24(left, left_offset) as usize;
    let right_key_size = read_u24(right, right_offset) as usize;

    if left.len() - left_offset - 3 < left_key_size
        || right.len() - right_offset - 3 < right_key_size
    {
        panic!("record key has invalid user key size");
    }

    let left_to = left_offset + 3 + left_key_size;
    let right_to = right_offset + 3 + right_key_size;

    let left_key: &[u8] = &left[(left_offset + 3)..left_to];
    let right_key: &[u8] = &right[(right_offset + 3)..right_to];

    let ord = left_key.cmp(right_key);

    (ord, left_to, right_to)
}

pub fn record_key_compare_byte_sized(
    left: &[u8],
    right: &[u8],
    left_offset: usize,
    right_offset: usize,
) -> (Ordering, usize, usize) {
    let left_size = left[left_offset] as usize;
    let right_size = right[right_offset] as usize;

    if left.len() - left_offset - 1 < left_size || right.len() - right_offset - 1 < right_size {
        panic!("record key single-byte invalid size");
    }

    let left_to = left_offset + 1 + left_size;
    let right_to = right_offset + 1 + right_size;

    let left_val: &[u8] = &left[(left_offset + 1)..left_to];
    let right_val: &[u8] = &right[(right_offset + 1)..right_to];

    let ord = left_val.cmp(right_val);

    (ord, left_to, right_to)
}

pub fn record_key_compare_fn(left: &[u8], right: &[u8]) -> Ordering {
    let left_length = left.len();
    let right_length = right.len();

    if left_length < MIN_KEY_SIZE || right_length < MIN_KEY_SIZE {
        panic!("record key less than minimum");
    }

    // WARN: first byte is ignored in compare, it stores flags that we don't want to place in the value

    let (ord, left_to, right_to) = record_key_compare_u24_sized(left, right, 1, 1);

    match ord {
        Ordering::Equal => {}
        found => {
            return found;
        }
    }

    if left_to >= left_length || right_to >= right_length {
        panic!("record key no generation id");
    }

    let (ord, left_to, right_to) = record_key_compare_byte_sized(left, right, left_to, right_to);

    match ord {
        Ordering::Equal => {}
        found => {
            return found;
        }
    }

    if left_to >= left_length || right_to >= right_length {
        panic!("record key no phantom id");
    }

    let (ord, _, _) = record_key_compare_byte_sized(left, right, left_to, right_to);

    ord
}
