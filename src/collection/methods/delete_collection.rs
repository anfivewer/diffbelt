use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::messages::generations::{
    DatabaseCollectionGenerationsTask, DropCollectionGenerationsTask,
};
use crate::raw_db::RawDb;
use crate::util::async_sync_call::async_sync_call;
use crate::util::tokio::spawn_blocking_async;
use std::future::Future;
use std::ops::DerefMut;
use std::path::PathBuf;

impl Collection {
    pub fn delete_collection(&self) -> impl Future<Output = Result<(), CollectionMethodError>> {
        let is_deleted = self.is_deleted.clone();
        let collection_name = self.name.clone();
        let collection_generations_id = self.generations_id.clone();
        let database_inner = self.database_inner.clone();
        let raw_db = self.raw_db.clone();

        let join = spawn_blocking_async(async move {
            // Make all methods return `NoSuchCollection` after this write
            // if they have ref to this Collection
            let mut deletion_lock = is_deleted.write().await;
            if deletion_lock.to_owned() {
                return Err(CollectionMethodError::NoSuchCollection);
            }

            let is_deleted = deletion_lock.deref_mut();
            *is_deleted = true;

            // Preparation to delete
            database_inner
                .start_delete_collection(&collection_name)
                .await?;

            let _: () = async_sync_call(|sender| {
                database_inner.add_generations_task(
                    DatabaseCollectionGenerationsTask::DropCollection(
                        DropCollectionGenerationsTask {
                            collection_id: collection_generations_id,
                            sender,
                        },
                    ),
                )
            })
            .await?;

            database_inner
                .remove_readers_pointing_to_collection(collection_name.clone())
                .await?;

            // Destroy raw_db, remove files
            let path = raw_db.get_path().to_string();
            let mut is_alive_receiver = raw_db.get_is_alive_receiver();
            drop(raw_db);

            loop {
                let result = is_alive_receiver.changed().await;
                match result {
                    Ok(_) => {}
                    Err(_) => {
                        // error is possible only if it was droppped, so db should be dropped
                        break;
                    }
                }

                let is_alive = *is_alive_receiver.borrow();

                if !is_alive {
                    break;
                }
            }

            RawDb::destroy(&path)?;

            // TODO: do not delete, move it and then delete after a few hours/days
            //       as config says and give ability to restore it
            let path = PathBuf::from(path);
            let path = path.as_path();

            std::fs::remove_dir_all(path).or_else(|err| {
                match err.kind() {
                    std::io::ErrorKind::NotFound => {
                        return Ok(());
                    }
                    _ => {}
                }

                Err(CollectionMethodError::CannotDeleteRawDbPath(err))
            })?;

            // Finalization of deletion
            database_inner.finish_delete_collection_sync(&collection_name)?;

            drop(deletion_lock);

            Ok::<(), CollectionMethodError>(())
        });

        async move { join.await.map_err(|_| CollectionMethodError::TaskJoin)? }
    }
}
