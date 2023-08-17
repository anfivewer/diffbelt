use crate::util::bytes::{from_u32_be, to_u32_be_unchecked};
use rocksdb::MergeOperands;

pub fn generation_size_full_merge(
    _key: &[u8],
    value: Option<&[u8]>,
    ops: &MergeOperands,
) -> Option<Vec<u8>> {
    let mut count = match value {
        Some(bytes) => to_u32_be_unchecked(bytes),
        None => 0,
    };

    for op in ops {
        count += to_u32_be_unchecked(op);
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
        count += to_u32_be_unchecked(op);
    }

    Some(Vec::from(from_u32_be(count)))
}
