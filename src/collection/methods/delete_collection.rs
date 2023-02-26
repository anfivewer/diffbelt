use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::raw_db::{RawDb, RawDbError};
use crate::util::tokio::spawn_blocking_async;
use std::future::Future;
use std::ops::DerefMut;
use std::path::PathBuf;

impl Collection {
    pub fn delete_collection(&self) -> impl Future<Output = Result<(), CollectionMethodError>> {
        let is_deleted = self.is_deleted.clone();
        let collection_name = self.name.clone();
        let database_inner = self.database_inner.clone();
        let raw_db = self.raw_db.clone();
        let newgen = self.newgen.clone();

        let join = spawn_blocking_async(async move {
            // Make all methods return `NoSuchCollection` after this write
            // if they have ref to this Collection
            let mut deletion_lock = is_deleted.write().await;
            if deletion_lock.to_owned() {
                return Err(CollectionMethodError::NoSuchCollection);
            }

            let is_deleted = deletion_lock.deref_mut();
            *is_deleted = true;

            {
                let mut newgen_lock = newgen.write().await;
                let newgen = newgen_lock.take();
                match newgen {
                    Some(mut newgen) => {
                        newgen.stop();
                    }
                    None => {}
                }
            }

            // Preparation to delete
            database_inner
                .start_delete_collection(&collection_name)
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

    fn destroy(&self) -> Result<(), RawDbError> {
        Ok(())
    }
}
