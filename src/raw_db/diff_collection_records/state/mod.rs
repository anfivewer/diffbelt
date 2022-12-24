use crate::collection::util::generation_key::{GenerationKey, OwnedGenerationKey};
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::{
    CollectionKey, GenerationId, IsByteArray, IsByteArrayMut, OwnedCollectionKey, OwnedGenerationId,
};
use crate::raw_db::diff_collection_records::DiffCursorState;
use crate::raw_db::RawDbError;
use crate::util::bytes::{increment, to_u32_be_unchecked};
use rocksdb::{BoundColumnFamily, Direction, IteratorMode, ReadOptions};
use std::collections::BTreeSet;
use std::sync::Arc;

mod diff;
pub mod in_memory;
pub mod single_generation;

/**
 *  This constant used as follows:
 *    - we try to load in memory this number of changed collection keys
 *    - if we are loaded at least one generation, then process them from memory
 *      (and actual `to_generation_id` will be generation on which we are accumulated enough keys)
 *    - if first generation has more keys than this constant says, then we are working in
 *      "iterator over db keys" mode
 *
 *  Later we'll should tune our puts to have at most key updates count as they can in adequate time,
 *  and then merge changed keys from N generations to increase possible number of items in the
 *  single diff. And also save iterated small-size generations to fictive range-generations.
 */
const TOTAL_COUNT_IN_GENERATIONS_LIMIT: u32 = 4000;
const RECORDS_TO_VIEW_LIMIT: usize = 2000;

pub struct DiffStateInMemoryMode {
    pub changed_keys: BTreeSet<OwnedCollectionKey>,
}

pub enum DiffStateMode {
    InMemory(DiffStateInMemoryMode),
    // `to_generation_id`
    SingleGeneration,
}

pub struct PrevDiffState {
    first_value: Option<Box<[u8]>>,
    last_value: Option<Box<[u8]>>,
    next_record_key: OwnedRecordKey,
}

pub struct DiffState<'a> {
    db: &'a rocksdb::DB,
    from_generation_id: GenerationId<'a>,
    to_generation_id: OwnedGenerationId,
    prev_state: Option<PrevDiffState>,
    records_to_view_left: usize,
    pack_limit: usize,
}

pub enum DiffStateNewResult<'a> {
    State((DiffState<'a>, DiffStateMode)),
    Empty,
}

impl<'a> DiffState<'a> {
    pub fn new(
        db: &'a rocksdb::DB,
        from_generation_id: GenerationId<'a>,
        to_generation_id_loose: GenerationId<'a>,
        pack_limit: usize,
    ) -> Result<DiffStateNewResult<'a>, RawDbError> {
        let generations_cf = db.cf_handle("gens").ok_or(RawDbError::CfHandle)?;
        let generations_size_cf = db.cf_handle("gens_size").ok_or(RawDbError::CfHandle)?;

        let mut upper_generation_key = to_generation_id_loose.to_owned();
        let upper_generation_key_bytes = upper_generation_key.get_byte_array_mut();
        increment(upper_generation_key_bytes);

        let iterator_mode =
            IteratorMode::From(from_generation_id.get_byte_array(), Direction::Forward);
        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(upper_generation_key_bytes);

        let mut iterator = db.iterator_cf_opt(&generations_size_cf, opts, iterator_mode);

        let mut total_count = 0;
        let mut to_generation_id;

        // Pick first generation, decide about mode
        let iterator_ref = iterator.by_ref();
        loop {
            let result = iterator_ref.next();
            let result = match result {
                Some(result) => result,
                None => {
                    return Ok(DiffStateNewResult::Empty);
                }
            };

            let (key, value) = result?;

            if key.as_ref() <= from_generation_id.get_byte_array() {
                continue;
            }

            if key.as_ref() > to_generation_id_loose.get_byte_array() {
                return Ok(DiffStateNewResult::Empty);
            }

            let count = to_u32_be_unchecked(&value);

            if count > TOTAL_COUNT_IN_GENERATIONS_LIMIT {
                return Ok(DiffStateNewResult::State((
                    DiffState {
                        db,
                        from_generation_id,
                        to_generation_id: OwnedGenerationId::from_boxed_slice(key),
                        prev_state: None,
                        records_to_view_left: RECORDS_TO_VIEW_LIMIT,
                        pack_limit,
                    },
                    DiffStateMode::SingleGeneration,
                )));
            }

            total_count += count;
            to_generation_id = OwnedGenerationId::from_boxed_slice(key);
            break;
        }

        for result in iterator {
            let (key, value): (Box<[u8]>, Box<[u8]>) = result?;

            let generation_id = OwnedGenerationId::from_boxed_slice(key);

            if generation_id.as_ref() > to_generation_id_loose {
                break;
            }

            let count = to_u32_be_unchecked(&value);

            if total_count + count > TOTAL_COUNT_IN_GENERATIONS_LIMIT {
                break;
            }

            total_count += count;
            to_generation_id = generation_id;
        }

        let keys = collect_changed_keys(
            db,
            generations_cf,
            from_generation_id,
            to_generation_id.as_ref(),
            None,
        )?;

        Ok(DiffStateNewResult::State((
            DiffState {
                db,
                from_generation_id,
                to_generation_id,
                prev_state: None,
                records_to_view_left: RECORDS_TO_VIEW_LIMIT,
                pack_limit,
            },
            DiffStateMode::InMemory(DiffStateInMemoryMode { changed_keys: keys }),
        )))
    }

    pub fn continue_prev(
        db: &'a rocksdb::DB,
        from_generation_id: GenerationId<'a>,
        to_generation_id: GenerationId<'a>,
        prev_state: DiffCursorState,
        pack_limit: usize,
    ) -> Result<DiffStateNewResult<'a>, RawDbError> {
        let generations_cf = db.cf_handle("gens").ok_or(RawDbError::CfHandle)?;

        let DiffCursorState {
            changed_key,
            first_value,
            last_value,
            next_record_key,
        } = prev_state;

        let keys = collect_changed_keys(
            db,
            generations_cf,
            from_generation_id,
            to_generation_id,
            Some(changed_key.as_ref()),
        )?;

        Ok(DiffStateNewResult::State((
            DiffState {
                db,
                from_generation_id,
                to_generation_id: to_generation_id.to_owned(),
                prev_state: Some(PrevDiffState {
                    first_value,
                    last_value,
                    next_record_key,
                }),
                records_to_view_left: RECORDS_TO_VIEW_LIMIT,
                pack_limit,
            },
            DiffStateMode::InMemory(DiffStateInMemoryMode { changed_keys: keys }),
        )))
    }

    pub fn get_to_generation_id(&self) -> GenerationId<'_> {
        self.to_generation_id.as_ref()
    }

    pub fn get_from_collection_key(&self) -> Option<CollectionKey<'_>> {
        self.prev_state
            .as_ref()
            .map(|prev_state: &PrevDiffState| prev_state.next_record_key.get_collection_key())
    }
}

fn collect_changed_keys(
    db: &rocksdb::DB,
    generations_cf: Arc<BoundColumnFamily<'_>>,
    from_generation_id: GenerationId<'_>,
    to_generation_id: GenerationId<'_>,
    filter_keys_less_than: Option<CollectionKey<'_>>,
) -> Result<BTreeSet<OwnedCollectionKey>, RawDbError> {
    let mut keys = BTreeSet::new();

    let iterator = {
        let from_generation_key =
            OwnedGenerationKey::new(from_generation_id, CollectionKey::empty())
                .or(Err(RawDbError::InvalidGenerationKey))?;

        let to_generation_key = {
            let mut to_generation_id_incremented = to_generation_id.to_owned();
            let to_generation_id_bytes = to_generation_id_incremented.get_byte_array_mut();
            increment(to_generation_id_bytes);
            OwnedGenerationKey::new(
                to_generation_id_incremented.as_ref(),
                CollectionKey::empty(),
            )
            .or(Err(RawDbError::InvalidGenerationKey))?
        };

        let iterator_mode =
            IteratorMode::From(from_generation_key.get_byte_array(), Direction::Forward);
        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(to_generation_key.get_byte_array());
        db.iterator_cf_opt(&generations_cf, opts, iterator_mode)
    };

    for result in iterator {
        let (key, _): (Box<[u8]>, Box<[u8]>) = result?;

        let generation_key =
            GenerationKey::validate(&key).or(Err(RawDbError::InvalidGenerationKey))?;

        // TODO: implement `parse()` method
        let generation_id = generation_key.get_generation_id();
        let collection_key = generation_key.get_collection_key();

        match filter_keys_less_than {
            Some(from_key) => {
                if collection_key < from_key {
                    continue;
                }
            }
            None => {}
        }

        if generation_id <= from_generation_id {
            continue;
        }

        if generation_id > to_generation_id {
            break;
        }

        keys.insert(collection_key.to_owned());
    }

    Ok(keys)
}
