use crate::collection::constants::COLLECTION_META_PREV_PHANTOM_ID_KEY;
use rocksdb::MergeOperands;

/**
    For the future me: partial merge has `value: None` and can return `None`, which indicates that
    partial merge is not supported.

    Full merge `value` is real previous value and **should** return `Some`, else operation will
    fail.
*/

pub fn meta_full_merge(key: &[u8], value: Option<&[u8]>, ops: &MergeOperands) -> Option<Vec<u8>> {
    meta_partial_merge(key, value, ops)
}

pub fn meta_partial_merge(
    key: &[u8],
    value: Option<&[u8]>,
    ops: &MergeOperands,
) -> Option<Vec<u8>> {
    if key == COLLECTION_META_PREV_PHANTOM_ID_KEY {
        // always write bigger phantom id
        const EMPTY_SLICE: &[u8] = &[];
        let max = value.iter().map(|x| *x).chain(ops.iter()).max();
        return max.map(|x| x.to_vec());
    }

    // When adding reader, use already written value if present or first one
    value.or_else(|| ops.iter().next()).map(|x| x.to_vec())
}
