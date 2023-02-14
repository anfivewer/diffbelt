use crate::collection::methods::errors::CollectionMethodError;

use crate::collection::constants::COLLECTION_GET_KEYS_AROUND_MAX_LIMIT;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::collection::Collection;

use crate::common::{OwnedCollectionKey, OwnedGenerationId, OwnedPhantomId};
use crate::raw_db::get_keys_around::{RawDbGetKeysAroundOptions, RawDbGetKeysAroundResult};

pub struct CollectionGetKeysAroundOptions {
    pub key: OwnedCollectionKey,
    pub generation_id: Option<OwnedGenerationId>,
    pub phantom_id: Option<OwnedPhantomId>,
    pub require_key_existance: bool,
    pub limit: usize,
}

#[derive(Debug)]
pub struct CollectionGetKeysAroundOk {
    pub generation_id: OwnedGenerationId,
    pub left: Vec<OwnedCollectionKey>,
    pub right: Vec<OwnedCollectionKey>,
    pub has_more_on_the_left: bool,
    pub has_more_on_the_right: bool,
}

impl Collection {
    pub async fn get_keys_around(
        &self,
        options: CollectionGetKeysAroundOptions,
    ) -> Result<CollectionGetKeysAroundOk, CollectionMethodError> {
        if !options.require_key_existance {
            return Err(CollectionMethodError::NotImplementedYet);
        }

        let limit = options.limit.min(COLLECTION_GET_KEYS_AROUND_MAX_LIMIT);
        let records_to_view_limit = self.config.query_pack_records_limit;

        let generation_id = self.generation_id_or_current(options.generation_id).await;

        let record_key = OwnedRecordKey::new(
            options.key.as_ref(),
            generation_id.as_ref(),
            OwnedPhantomId::or_empty_as_ref(&options.phantom_id),
        )
        .or(Err(CollectionMethodError::InvalidKey))?;

        let deletion_lock = self.is_deleted.read().await;
        if *deletion_lock {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = {
            let db = self.raw_db.clone();
            tokio::task::spawn_blocking(move || {
                db.keys_around_sync(RawDbGetKeysAroundOptions {
                    record_key: record_key.as_ref(),
                    limit,
                    records_to_view_limit,
                })
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??
        };

        drop(deletion_lock);

        let RawDbGetKeysAroundResult {
            left,
            right,
            has_more_on_the_left,
            has_more_on_the_right,
        } = result;

        Ok(CollectionGetKeysAroundOk {
            generation_id,
            left,
            right,
            has_more_on_the_left,
            has_more_on_the_right,
        })
    }
}
