use rocksdb::MergeOperands;

pub fn meta_full_merge(_key: &[u8], value: Option<&[u8]>, ops: &MergeOperands) -> Option<Vec<u8>> {
    let first_op = ops.iter().next();

    let new_value = match first_op {
        None => {
            // Bad merge, no operand, no change
            return value.map(|bytes| bytes.to_vec());
        }
        Some(value) => value,
    };

    match value {
        None => {
            // No value stored, save first operand
            return Some(new_value.to_vec());
        }
        // Some value already present, do not change it
        Some(value) => Some(value.to_vec()),
    }
}

pub fn meta_partial_merge(
    _key: &[u8],
    value: Option<&[u8]>,
    ops: &MergeOperands,
) -> Option<Vec<u8>> {
    assert!(value.is_some());

    let first_op = ops.iter().next();

    first_op.map(|bytes| bytes.to_vec())
}
