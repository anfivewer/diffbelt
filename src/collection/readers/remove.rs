use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::messages::readers::{DatabaseCollecitonReadersTask, DeleteReaderTask};
use crate::raw_db::update_reader::RawDbDeleteReaderOptions;
use std::sync::Arc;
use tokio::task::spawn_blocking;

impl Collection {
    pub async fn inner_remove_reader(
        &self,
        reader_name: Arc<str>,
    ) -> Result<(), CollectionMethodError> {
        let raw_db = self.raw_db.clone();

        let reader_name_for_blocking = reader_name.clone();

        spawn_blocking(move || {
            raw_db.delete_reader_sync(RawDbDeleteReaderOptions {
                reader_name: reader_name_for_blocking.as_ref(),
            })
        })
        .await
        .map_err(|_| CollectionMethodError::TaskJoin)?
        .map_err(CollectionMethodError::RawDb)?;

        self.database_inner
            .add_readers_task(DatabaseCollecitonReadersTask::DeleteReader(
                DeleteReaderTask {
                    owner_collection_name: self.name.clone(),
                    reader_name,
                },
            ))
            .await;

        Ok(())
    }
}
