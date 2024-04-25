use crate::util::bytes::{from_u32_be, read_u32_be};
use rocksdb::MergeOperands;

pub fn generation_size_full_merge(
    _key: &[u8],
    value: Option<&[u8]>,
    ops: &MergeOperands,
) -> Option<Vec<u8>> {
    let mut count = match value {
        Some(bytes) => read_u32_be(bytes),
        None => 0,
    };

    for op in ops {
        count += read_u32_be(op);
    }

    Some(Vec::from(from_u32_be(count)))
}

pub fn generation_size_partial_merge(
    _key: &[u8],
    value: Option<&[u8]>,
    ops: &MergeOperands,
) -> Option<Vec<u8>> {
    assert!(value.is_none());

    let mut count = 0;

    for op in ops {
        count += read_u32_be(op);
    }

    Some(Vec::from(from_u32_be(count)))
}
