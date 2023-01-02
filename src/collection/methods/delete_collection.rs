use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::raw_db::destroy::DestroyOk;
use crate::raw_db::RawDbError;
use std::ops::DerefMut;
use std::path::PathBuf;

impl Collection {
    pub async fn delete_collection(&self) -> Result<(), CollectionMethodError> {
        // Make all methods return `NoSuchCollection` after this write
        // if they have ref to this Collection
        let mut deletion_lock = self.is_deleted.write().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let is_deleted = deletion_lock.deref_mut();
        *is_deleted = true;

        {
            let collection_id = self.id.clone();
            let database_inner = self.database_inner.clone();
            let raw_db = self.raw_db.clone();

            tokio::task::spawn_blocking(move || {
                // Preparation to delete
                database_inner.start_delete_collection_sync(&collection_id)?;

                // Destroy raw_db, remove files
                let DestroyOk { path } = raw_db.destroy()?;

                // TODO: do not delete, move it and then delete after a few hours/days
                //       as config says and give ability to restore it
                let path = PathBuf::from(path);
                let path = path.as_path();

                println!("REMOVE {:?}", path.to_str());

                std::fs::remove_dir_all(path)
                    .or(Err(CollectionMethodError::CannotDeleteRawDbPath))?;

                // Finalization of deletion
                database_inner.finish_delete_collection_sync(&collection_id)?;

                Ok::<(), CollectionMethodError>(())
            })
            .await
            .or(Err(CollectionMethodError::TaskJoin))??;
        }

        drop(deletion_lock);

        Ok(())
    }

    fn destroy(&self) -> Result<(), RawDbError> {
        Ok(())
    }
}
