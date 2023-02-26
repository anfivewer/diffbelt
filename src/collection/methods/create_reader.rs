use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::{GenerationId, OwnedGenerationId};

use crate::raw_db::update_reader::{RawDbCreateReaderOptions, RawDbCreateReaderResult};

use crate::util::tokio::spawn_blocking_async;

pub struct CreateReaderOptions {
    pub reader_name: String,
    pub collection_name: Option<String>,
    pub generation_id: Option<OwnedGenerationId>,
}

impl Collection {
    pub async fn create_reader(
        &self,
        options: CreateReaderOptions,
    ) -> Result<(), CollectionMethodError> {
        let reader_name = options.reader_name;
        let collection_name = options.collection_name;
        let generation_id = options.generation_id;
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = spawn_blocking_async(async move {
            raw_db.create_reader_sync(RawDbCreateReaderOptions {
                reader_name: reader_name.as_str(),
                collection_name: collection_name.as_ref().map(|id| id.as_str()),
                generation_id: GenerationId::from_opt_owned(&generation_id),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        match result {
            RawDbCreateReaderResult::Created => Ok(()),
            RawDbCreateReaderResult::AlreadyExists(reader_value) => {
                Err(CollectionMethodError::ReaderAlreadyExists(reader_value))
            }
        }
    }
}
