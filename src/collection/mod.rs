use crate::common::GenerationId;
use crate::context::Context;
use crate::generation::CollectionGeneration;
use crate::raw_db::{
    RawDb, RawDbColumnFamily, RawDbComparator, RawDbOpenError as RawDbOpenErrorExternal,
    RawDbOptions,
};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

mod methods;

type ReaderCollectionId<'a> = &'a str;
type ReaderId<'a> = &'a str;

pub type GetReaderGenerationIdFn =
    Box<dyn Fn(ReaderCollectionId<'_>, ReaderId<'_>) -> GenerationId>;

pub struct Collection {
    id: String,
    raw_db: Arc<RawDb>,
    is_manual: bool,
    generation_id: RwLock<RefCell<GenerationId>>,
    // None if this is manual collection and generation is not yet started
    // in non-manual collections always present
    next_generation: RwLock<RefCell<Option<CollectionGeneration>>>,
    get_reader_generation_id: GetReaderGenerationIdFn,
}

pub struct NewCollectionOptions {
    pub id: String,
    pub context: Arc<RwLock<Context>>,
    pub is_manual: bool,
}

pub enum CollectionOpenError {
    PathJoinError,
    RawDbOpenError(RawDbOpenErrorExternal),
}

impl From<RawDbOpenErrorExternal> for CollectionOpenError {
    fn from(err: RawDbOpenErrorExternal) -> Self {
        CollectionOpenError::RawDbOpenError(err)
    }
}

/*
    1 -- reserved byte
    3 -- size of user key
    1 -- size of generationId
    1 -- size of phantomId
*/
const MIN_KEY_SIZE: usize = 1 + 3 + 1 + 1;

fn read_u24(bytes: &[u8], offset: usize) -> u32 {
    ((bytes[offset + 2] as u32) << 16) + ((bytes[offset + 1] as u32) << 8) + (bytes[offset] as u32)
}

fn record_key_compare_byte_sized(
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

fn record_key_compare_fun(left: &[u8], right: &[u8]) -> Ordering {
    let left_length = left.len();
    let right_length = right.len();

    if left_length < MIN_KEY_SIZE || right_length < MIN_KEY_SIZE {
        panic!("record key less than minimum");
    }

    if left[0] != 0 || right_length != 0 {
        panic!("record key reserved flag byte is not zero");
    }

    let left_key_size = read_u24(left, 1) as usize;
    let right_key_size = read_u24(right, 1) as usize;

    if left_length - MIN_KEY_SIZE < left_key_size || right_length - MIN_KEY_SIZE < right_key_size {
        panic!("record key has invalid user key size");
    }

    let left_to = 4 + left_key_size;
    let right_to = 4 + right_key_size;

    let left_key: &[u8] = &left[4..left_to];
    let right_key: &[u8] = &right[4..right_to];

    let ord = left_key.cmp(right_key);
    match ord {
        Ordering::Equal => {}
        found => {
            return found;
        }
    }

    let (ord, left_to, right_to) = record_key_compare_byte_sized(left, right, left_to, right_to);

    match ord {
        Ordering::Equal => {}
        found => {
            return found;
        }
    }

    let (ord, _, _) = record_key_compare_byte_sized(left, right, left_to, right_to);

    ord
}

impl Collection {
    // Create a new column family descriptor with the specified name and options.
    pub async fn open(options: NewCollectionOptions) -> Result<Self, CollectionOpenError> {
        let collection_id = options.id;

        let context = options.context.read().await;
        let path = Path::new(&context.config.data_path).join(collection_id);
        let path = path.to_str().ok_or(CollectionOpenError::PathJoinError)?;
        drop(context);

        let raw_db = RawDb::open_raw_db(RawDbOptions {
            path,
            comparator: Some(RawDbComparator {
                name: "v1".to_string(),
                compare_fn: record_key_compare_fun,
            }),
            column_families: vec![
                RawDbColumnFamily {
                    name: "gens".to_string(),
                    comparator: None,
                },
                RawDbColumnFamily {
                    name: "meta".to_string(),
                    comparator: None,
                },
            ],
        })?;

        let meta = raw_db.with_cf("meta");

        todo!();
    }
}
