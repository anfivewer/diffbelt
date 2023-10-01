use crate::collection::constants::{COLLECTION_CF_GENERATIONS, COLLECTION_CF_GENERATIONS_SIZE};
use crate::collection::util::generation_key::{GenerationKey, OwnedGenerationKey};
use crate::collection::util::record_key::RecordKey;
use crate::common::{
    CollectionKey, GenerationId, IsByteArray, OwnedCollectionKey, OwnedGenerationId,
};
use crate::raw_db::diff_collection_records::DiffCursorState;
use crate::raw_db::RawDbError;
use crate::util::bytes::to_u32_be_unchecked;
use diffbelt_util::cast::u32_to_usize;
use rocksdb::{BoundColumnFamily, Direction, IteratorMode, ReadOptions};
use std::collections::BTreeSet;
use std::sync::Arc;

mod diff;
pub mod in_memory;
pub mod single_generation;

pub struct DiffStateInMemoryMode {
    pub changed_keys: BTreeSet<OwnedCollectionKey>,
}

pub enum DiffStateMode {
    InMemory(DiffStateInMemoryMode),
    // `to_generation_id`
    SingleGeneration,
}

pub struct PrevDiffState<'a> {
    first_value: Option<&'a [u8]>,
    last_value: Option<&'a [u8]>,
    next_record_key: RecordKey<'a>,
}

pub struct DiffState<'a> {
    db: &'a rocksdb::DB,
    from_generation_id: Option<GenerationId<'a>>,
    to_generation_id: OwnedGenerationId,
    prev_state: Option<PrevDiffState<'a>>,
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
        from_generation_id: Option<GenerationId<'a>>,
        to_generation_id_loose: GenerationId<'a>,
        pack_limit: usize,
        records_to_view_limit: usize,
        total_count_in_generations_limit: usize,
    ) -> Result<DiffStateNewResult<'a>, RawDbError> {
        let generations_cf = db
            .cf_handle(COLLECTION_CF_GENERATIONS)
            .ok_or(RawDbError::CfHandle)?;
        let generations_size_cf = db
            .cf_handle(COLLECTION_CF_GENERATIONS_SIZE)
            .ok_or(RawDbError::CfHandle)?;

        let upper_generation_key = to_generation_id_loose.incremented();

        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(upper_generation_key.get_byte_array());

        let mut iterator = match from_generation_id {
            Some(id) => {
                let iterator_mode = IteratorMode::From(id.get_byte_array(), Direction::Forward);
                db.iterator_cf_opt(&generations_size_cf, opts, iterator_mode)
            }
            None => db.iterator_cf_opt(&generations_size_cf, opts, IteratorMode::Start),
        };

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

            let generation_id = OwnedGenerationId::from_boxed_slice(key)
                .or(Err(RawDbError::InvalidGenerationId))?;

            if generation_id
                .as_ref()
                .less_or_equal_with_opt_or(from_generation_id, false)
            {
                continue;
            }

            if generation_id.as_ref() > to_generation_id_loose {
                return Ok(DiffStateNewResult::Empty);
            }

            let count = u32_to_usize(to_u32_be_unchecked(&value));

            if count > total_count_in_generations_limit {
                return Ok(DiffStateNewResult::State((
                    DiffState {
                        db,
                        from_generation_id,
                        to_generation_id: generation_id,
                        prev_state: None,
                        records_to_view_left: records_to_view_limit,
                        pack_limit,
                    },
                    DiffStateMode::SingleGeneration,
                )));
            }

            total_count += count;
            to_generation_id = generation_id;
            break;
        }

        for result in iterator {
            let (key, value): (Box<[u8]>, Box<[u8]>) = result?;

            let generation_id = OwnedGenerationId::from_boxed_slice(key)
                .or(Err(RawDbError::InvalidGenerationId))?;

            if generation_id.as_ref() > to_generation_id_loose {
                break;
            }

            let count = u32_to_usize(to_u32_be_unchecked(&value));

            if total_count + count > total_count_in_generations_limit {
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
                records_to_view_left: records_to_view_limit,
                pack_limit,
            },
            DiffStateMode::InMemory(DiffStateInMemoryMode { changed_keys: keys }),
        )))
    }

    pub fn continue_prev(
        db: &'a rocksdb::DB,
        from_generation_id: Option<GenerationId<'a>>,
        to_generation_id: GenerationId<'a>,
        prev_state: &'a DiffCursorState,
        pack_limit: usize,
        records_to_view_limit: usize,
    ) -> Result<DiffStateNewResult<'a>, RawDbError> {
        let generations_cf = db
            .cf_handle(COLLECTION_CF_GENERATIONS)
            .ok_or(RawDbError::CfHandle)?;

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
                    first_value: first_value
                        .as_ref()
                        .map(|bytes| AsRef::<[u8]>::as_ref(bytes)),
                    last_value: last_value
                        .as_ref()
                        .map(|bytes| AsRef::<[u8]>::as_ref(bytes)),
                    next_record_key: next_record_key.as_ref(),
                }),
                records_to_view_left: records_to_view_limit,
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
    from_generation_id: Option<GenerationId<'_>>,
    to_generation_id: GenerationId<'_>,
    filter_keys_less_than: Option<CollectionKey<'_>>,
) -> Result<BTreeSet<OwnedCollectionKey>, RawDbError> {
    let mut keys = BTreeSet::new();

    let iterator = {
        let to_generation_key = {
            let to_generation_id_incremented = to_generation_id.incremented();

            OwnedGenerationKey::new(
                to_generation_id_incremented.as_ref(),
                CollectionKey::empty(),
            )
            .or(Err(RawDbError::InvalidGenerationKey))?
        };

        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(to_generation_key.get_byte_array());

        match from_generation_id {
            Some(from_generation_id) => {
                let from_generation_key =
                    OwnedGenerationKey::new(from_generation_id, CollectionKey::empty())
                        .or(Err(RawDbError::InvalidGenerationKey))?;

                let iterator_mode =
                    IteratorMode::From(from_generation_key.get_byte_array(), Direction::Forward);

                db.iterator_cf_opt(&generations_cf, opts, iterator_mode)
            }
            None => db.iterator_cf_opt(&generations_cf, opts, IteratorMode::Start),
        }
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

        if generation_id.less_or_equal_with_opt_or(from_generation_id, false) {
            continue;
        }

        if generation_id > to_generation_id {
            break;
        }

        keys.insert(collection_key.to_owned());
    }

    Ok(keys)
}
