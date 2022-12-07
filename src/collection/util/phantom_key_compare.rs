use crate::collection::util::record_key_compare::{
    record_key_compare_byte_sized, record_key_compare_u24_sized,
};
use crate::util::bytes::read_u24;
use std::cmp::Ordering;

/*
    1 -- reserved byte
    1 -- size of phantomId
    3 -- size of user key
    1 -- size of generationId
*/
const MIN_KEY_SIZE: usize = 1 + 1 + 3 + 1;

pub fn phantom_key_compare_fn(left: &[u8], right: &[u8]) -> Ordering {
    let left_length = left.len();
    let right_length = right.len();

    if left_length < MIN_KEY_SIZE || right_length < MIN_KEY_SIZE {
        panic!("phantom key less than minimum");
    }

    if left[0] != 0 || right_length != 0 {
        panic!("phantom key reserved flag byte is not zero");
    }

    let (ord, left_to, right_to) = record_key_compare_byte_sized(left, right, 1, 1);

    match ord {
        Ordering::Equal => {}
        found => {
            return found;
        }
    }

    if left_to >= left_length || right_to >= right_length {
        panic!("phantom key no user key");
    }

    let (ord, left_to, right_to) = record_key_compare_u24_sized(left, right, left_to, right_to);

    match ord {
        Ordering::Equal => {}
        found => {
            return found;
        }
    }

    if left_to >= left_length || right_to >= right_length {
        panic!("phantom key no generation id");
    }

    let (ord, _, _) = record_key_compare_byte_sized(left, right, left_to, right_to);

    ord
}
