use crate::collection::constants::COLLECTION_CF_META;
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{IsByteArray, IsByteArrayMut, OwnedPhantomId};

use crate::util::bytes::increment;

use crate::util::tokio::spawn_blocking_async;

impl Collection {
    pub async fn start_phantom(&self) -> Result<OwnedPhantomId, CollectionMethodError> {
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let next_phantom_id = {
            let mut prev_phantom_id_lock = self.prev_phantom_id.write().await;
            increment(prev_phantom_id_lock.get_byte_array_mut());
            prev_phantom_id_lock.to_owned()
        };

        let next_phantom_id_cloned = next_phantom_id.clone();

        spawn_blocking_async(async move {
            raw_db.put_cf_sync(
                COLLECTION_CF_META,
                b"prev_phantom_id",
                next_phantom_id_cloned.get_byte_array(),
            )
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        Ok(next_phantom_id)
    }
}
