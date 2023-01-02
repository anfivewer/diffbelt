use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::raw_db::update_reader::RawDbDeleteReaderOptions;

use crate::util::tokio::spawn_blocking_async;

pub struct DeleteReaderOptions {
    pub reader_id: String,
}

impl Collection {
    pub async fn delete_reader(
        &self,
        options: DeleteReaderOptions,
    ) -> Result<(), CollectionMethodError> {
        if !self.is_manual {
            return Err(CollectionMethodError::UnsupportedOperationForThisCollectionType);
        }

        let reader_id = options.reader_id;
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = spawn_blocking_async(async move {
            raw_db.delete_reader_sync(RawDbDeleteReaderOptions {
                reader_id: reader_id.as_str(),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        Ok(result)
    }
}
