use crate::collection::constants::{COLLECTION_CF_META, COLLECTION_META_PREV_PHANTOM_ID_KEY};
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{IsByteArray, IsByteArrayMut, OwnedGenerationId, OwnedPhantomId};
use crate::raw_db::start_phantom::StartPhantomOptions as RawDbStartPhantomOptions;
use crate::util::bytes::increment;
use crate::util::tokio::spawn_blocking_async;

pub struct StartPhantomOptions {
    generation_id: OwnedGenerationId,
}

impl Collection {
    pub async fn start_phantom(&self) -> Result<OwnedPhantomId, CollectionMethodError> {
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let next_phantom_id = self.next_phantom_id().await;
        let next_phantom_id_cloned = next_phantom_id.clone();

        spawn_blocking_async(async move {
            raw_db.start_phantom_sync(RawDbStartPhantomOptions {
                phantom_id: next_phantom_id_cloned.as_ref(),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        Ok(next_phantom_id)
    }

    async fn next_phantom_id(&self) -> OwnedPhantomId {
        let mut prev_phantom_id_lock = self.prev_phantom_id.write().await;
        increment(prev_phantom_id_lock.get_byte_array_mut());
        prev_phantom_id_lock.to_owned()
    }
}
