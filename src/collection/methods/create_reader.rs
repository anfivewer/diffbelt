use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::raw_db::update_reader::{RawDbCreateReaderOptions, RawDbCreateReaderResult};

use crate::util::tokio::spawn_blocking_async;

pub struct CreateReaderOptions {
    pub reader_id: String,
    pub collection_id: Option<String>,
}

impl Collection {
    pub async fn create_reader(
        &self,
        options: CreateReaderOptions,
    ) -> Result<(), CollectionMethodError> {
        if !self.is_manual {
            return Err(CollectionMethodError::UnsupportedOperationForThisCollectionType);
        }

        let reader_id = options.reader_id;
        let collection_id = options.collection_id;
        let meta_raw_db = self.meta_raw_db.clone();

        let result = spawn_blocking_async(async move {
            meta_raw_db.create_reader_sync(RawDbCreateReaderOptions {
                reader_id: reader_id.as_str(),
                collection_id: collection_id.as_ref().map(|id| id.as_str()),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        match result {
            RawDbCreateReaderResult::Created => Ok(()),
            RawDbCreateReaderResult::AlreadyExists(reader_value) => {
                Err(CollectionMethodError::ReaderAlreadyExists(reader_value))
            }
        }
    }
}