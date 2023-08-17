use crate::collection::constants::COLLECTION_CF_GENERATIONS;
use crate::collection::util::generation_key::{GenerationKey, OwnedGenerationKey};
use crate::collection::util::record_key::{OwnedRecordKey, RecordKey};
use crate::common::{CollectionKey, GenerationId, IsByteArray, PhantomId};
use crate::raw_db::{RawDb, RawDbError};
use rocksdb::{Direction, IteratorMode, ReadOptions, WriteBatchWithTransaction, DB};
use std::cmp::Ordering;
use std::num::NonZeroUsize;

pub struct CleanupGenerationsLessThanOptions<'a> {
    pub generation_less_than: GenerationId<'a>,
    pub continue_from_record_key: Option<OwnedRecordKey>,
    pub records_limit: NonZeroUsize,
    pub lookups_limit: NonZeroUsize,
}

pub enum CleanupResult {
    NeedToContinue(Option<OwnedRecordKey>),
    Finished,
}

impl RawDb {
    pub fn cleanup_generations_less_than_sync(
        &self,
        options: CleanupGenerationsLessThanOptions<'_>,
    ) -> Result<CleanupResult, RawDbError> {
        let CleanupGenerationsLessThanOptions {
            generation_less_than,
            mut continue_from_record_key,
            records_limit,
            lookups_limit,
        } = options;

        let mut records_limit = records_limit.get();
        let mut lookups_limit = lookups_limit.get();

        let db = self.db.get_db();

        let generations_cf = db
            .cf_handle(COLLECTION_CF_GENERATIONS)
            .ok_or(RawDbError::CfHandle)?;

        let generation_keys_iterator = {
            let to_generation_key =
                OwnedGenerationKey::new(generation_less_than, CollectionKey::empty())
                    .map_err(|_| RawDbError::InvalidGenerationKey)?;

            let iterator_mode = IteratorMode::Start;
            let mut opts = ReadOptions::default();
            opts.set_iterate_upper_bound(to_generation_key.value);

            db.iterator_cf_opt(&generations_cf, opts, iterator_mode)
        };

        let mut result = CleanupResult::Finished;

        let mut batch = WriteBatchWithTransaction::<false>::default();

        'records_loop: for item in generation_keys_iterator {
            let (key, _) = item?;

            let generation_key =
                GenerationKey::validate(&key).map_err(|_| RawDbError::InvalidGenerationKey)?;

            let generation_id = generation_key.get_generation_id();

            if generation_id >= generation_less_than {
                break 'records_loop;
            }

            let collection_key = generation_key.get_collection_key();

            let continue_from_record_key = continue_from_record_key.take().and_then(|record_key| {
                if record_key.as_ref().get_collection_key() == collection_key {
                    Some(record_key)
                } else {
                    None
                }
            });

            let key_result = cleanup_collection_key(
                db,
                generation_less_than,
                collection_key,
                continue_from_record_key,
                &mut records_limit,
                &mut lookups_limit,
            )?;

            match key_result {
                CleanupCollectionKeyResult::Finished => {
                    batch.delete_cf(&generations_cf, key);
                }
                CleanupCollectionKeyResult::LimitReached(continue_from_record_key) => {
                    result = CleanupResult::NeedToContinue(Some(continue_from_record_key));
                    break 'records_loop;
                }
            }

            lookups_limit -= 1;
            if lookups_limit <= 0 {
                result = CleanupResult::NeedToContinue(None);
                break 'records_loop;
            }
        }

        let _: () = db.write(batch)?;

        Ok(result)
    }
}

enum CleanupCollectionKeyResult {
    Finished,
    LimitReached(OwnedRecordKey),
}

fn cleanup_collection_key(
    db: &DB,
    generation_less_than: GenerationId<'_>,
    collection_key: CollectionKey<'_>,
    continue_from_record_key: Option<OwnedRecordKey>,
    records_limit: &mut usize,
    lookups_limit: &mut usize,
) -> Result<CleanupCollectionKeyResult, RawDbError> {
    let records_iterator = {
        let record_key = {
            if let Some(record_key) = continue_from_record_key {
                record_key
            } else {
                OwnedRecordKey::new(collection_key, GenerationId::empty(), PhantomId::empty())
                    .map_err(|_| RawDbError::InvalidRecordKey)?
            }
        };

        let to_record_key =
            OwnedRecordKey::new(collection_key, generation_less_than, PhantomId::empty())
                .map_err(|_| RawDbError::InvalidRecordKey)?;

        let iterator_mode = IteratorMode::From(record_key.get_byte_array(), Direction::Forward);
        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(to_record_key.value);

        db.iterator(iterator_mode)
    };

    let mut result = CleanupCollectionKeyResult::Finished;

    let mut batch = WriteBatchWithTransaction::<false>::default();
    let mut prev_key = None;

    'records_loop: for item in records_iterator {
        let (key, _) = item?;

        let record_key = RecordKey::validate(&key).map_err(|_| RawDbError::InvalidRecordKey)?;
        let record_key_parsed = record_key.parse();

        if record_key_parsed.collection_key != collection_key {
            break 'records_loop;
        }

        if record_key_parsed.phantom_id.is_some() {
            *lookups_limit -= 1;
            if *lookups_limit <= 0 {
                result = CleanupCollectionKeyResult::LimitReached(record_key.to_owned());
                break 'records_loop;
            }

            continue;
        }

        let ord = record_key_parsed.generation_id.cmp(&generation_less_than);

        if ord == Ordering::Less || ord == Ordering::Equal {
            if let Some(prev_key) = prev_key.take() {
                // We should not delete last present record, so always remove previous one
                batch.delete(&prev_key);
            }
        }

        if ord == Ordering::Greater || ord == Ordering::Equal {
            break 'records_loop;
        }

        *records_limit -= 1;
        if *records_limit <= 0 {
            result = CleanupCollectionKeyResult::LimitReached(record_key.to_owned());
            break 'records_loop;
        }

        *lookups_limit -= 1;
        if *lookups_limit <= 0 {
            result = CleanupCollectionKeyResult::LimitReached(record_key.to_owned());
            break 'records_loop;
        }

        prev_key = Some(key);
    }

    let _: () = db.write(batch)?;

    Ok(result)
}
