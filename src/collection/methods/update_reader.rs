use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::OwnedGenerationId;
use crate::raw_db::update_reader::RawDbUpdateReaderOptions;

use crate::util::tokio::spawn_blocking_async;

pub struct UpdateReaderOptions {
    pub reader_name: String,
    pub generation_id: Option<OwnedGenerationId>,
}

impl Collection {
    pub async fn update_reader(
        &self,
        options: UpdateReaderOptions,
    ) -> Result<(), CollectionMethodError> {
        let reader_name = options.reader_name;
        let generation_id = options.generation_id;
        let raw_db = self.raw_db.clone();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = spawn_blocking_async(async move {
            raw_db.update_reader_sync(RawDbUpdateReaderOptions {
                reader_name: reader_name.as_str(),
                generation_id: generation_id.as_ref().map(|id| id.as_ref()),
            })
        })
        .await
        .or(Err(CollectionMethodError::TaskJoin))??;

        drop(deletion_lock);

        Ok(result)
    }
}
