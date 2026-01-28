use crate::update_ms::percentiles::reduce::parsed_intermediate_key::ParsedIntermediateKey;
use core::cmp::Ordering;

pub type KeysAround = (
    Vec<ParsedIntermediateKey>,
    ParsedIntermediateKey,
    Vec<ParsedIntermediateKey>,
);

/**
    Tries to insert key into KeysAround, if unsuccessful -- returns false
*/
pub fn insert_into_keys_around(keys_around: &mut KeysAround, key: &ParsedIntermediateKey) -> bool {
    let (left, center, right) = keys_around;

    let cmp = key.cmp(center as &ParsedIntermediateKey);

    let (side, need_reverse) = match cmp {
        Ordering::Less => (left, true),
        Ordering::Equal => {
            return true;
        }
        Ordering::Greater => (right, false),
    };

    let pos = side.binary_search_by(|k| {
        let cmp = key.cmp(k);
        let cmp = if need_reverse { cmp.reverse() } else { cmp };

        cmp
    });
    
    let pos = match pos {
        Ok(_) => {
            // already present
            return true;
        }
        Err(x) => x,
    };
    
    side.insert(pos, key.clone());

    todo!()
}
