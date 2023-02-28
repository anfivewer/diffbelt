use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use std::sync::Arc;

pub struct DeleteReaderOptions {
    pub reader_name: String,
}

impl Collection {
    pub async fn delete_reader(
        &self,
        options: DeleteReaderOptions,
    ) -> Result<(), CollectionMethodError> {
        let reader_name = Arc::from(options.reader_name);

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        self.inner_remove_reader(Arc::clone(&reader_name)).await?;

        drop(deletion_lock);

        Ok(())
    }
}
