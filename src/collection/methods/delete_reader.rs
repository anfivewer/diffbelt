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
        let meta_raw_db = self.meta_raw_db.clone();

        let result = spawn_blocking_async(async move {
            meta_raw_db.delete_reader_sync(RawDbDeleteReaderOptions {
                reader_id: reader_id.as_str(),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        Ok(result)
    }
}
