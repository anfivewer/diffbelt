use crate::collection::constants::{COLLECTION_CF_META, COLLECTION_META_PREV_PHANTOM_ID_KEY};
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{IsByteArray, IsByteArrayMut, OwnedGenerationId, OwnedPhantomId};
use crate::util::bytes::increment;
use crate::util::tokio::spawn_blocking_async;

pub struct StartPhantomOptions {
    generation_id: OwnedGenerationId,
}

impl Collection {
    async fn next_phantom_id(&self) -> OwnedPhantomId {
        let mut prev_phantom_id_lock = self.prev_phantom_id.write().await;
        increment(prev_phantom_id_lock.get_byte_array_mut());
        prev_phantom_id_lock.to_owned()
    }

    #[deprecated(note = "Use start_phantom_bound")]
    pub async fn start_phantom(&self) -> Result<OwnedPhantomId, CollectionMethodError> {
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let next_phantom_id = self.next_phantom_id().await;
        let next_phantom_id_cloned = next_phantom_id.clone();

        spawn_blocking_async(async move {
            raw_db.merge_cf_sync(
                COLLECTION_CF_META,
                COLLECTION_META_PREV_PHANTOM_ID_KEY,
                next_phantom_id_cloned.get_byte_array(),
            )
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        Ok(next_phantom_id)
    }

    pub async fn start_phantom_bound(
        &self,
        options: StartPhantomOptions,
    ) -> Result<OwnedPhantomId, CollectionMethodError> {
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let next_phantom_id = self.next_phantom_id().await;
        let next_phantom_id_cloned = next_phantom_id.clone();

        spawn_blocking_async(async move {
            raw_db.merge_cf_sync(
                COLLECTION_CF_META,
                COLLECTION_META_PREV_PHANTOM_ID_KEY,
                next_phantom_id_cloned.get_byte_array(),
            )
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        Ok(next_phantom_id)
    }
}
