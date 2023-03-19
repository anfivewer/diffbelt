#[cfg(test)]
use crate::collection::Collection;
#[cfg(test)]
use crate::messages::cursors::{DatabaseCollectionCursorsTask, GetCollectionQueryCursorsCountTask};
#[cfg(test)]
use crate::util::async_sync_call::async_sync_call;

#[cfg(test)]
impl Collection {
    pub async fn query_cursors_count(&self) -> usize {
        let count = async_sync_call(|sender| {
            self.database_inner.add_cursors_task(
                DatabaseCollectionCursorsTask::GetCollectionQueryCursorsCount(
                    GetCollectionQueryCursorsCountTask {
                        collection_id: self.cursors_id,
                        sender,
                    },
                ),
            )
        })
        .await
        .map_err(|_| ())
        .unwrap()
        .map_err(|_| ())
        .unwrap();

        count
    }
}
