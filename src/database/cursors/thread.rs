use crate::database::config::DatabaseConfig;
use crate::database::cursors::collection::InnerCursorsCollection;
use crate::database::cursors::storage::{CursorError, CursorType, InnerCursors};
use std::sync::Arc;

#[cfg(test)]
use crate::messages::cursors::GetCollectionCursorsCountTask;
use crate::messages::cursors::{
    AbortCursorTask, AddCursorContinuationTask, AddCursorTask, DatabaseCollectionCursorsTask,
    DatabaseCollectionSpecificCursorsTask, DropCollectionTask, FinishCursorTask,
    FullyFinishCursorTask, GetCursorByPublicIdTask, NewCollectionTask,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::indexed_container::IndexedContainer;

struct CursorsThreadState {
    config: Arc<DatabaseConfig>,
    collections: IndexedContainer<InnerCursorsCollection>,
}

pub async fn run(
    config: Arc<DatabaseConfig>,
    mut poller: TaskPoller<DatabaseCollectionCursorsTask>,
) {
    let mut state = CursorsThreadState {
        config,
        collections: IndexedContainer::new(),
    };

    while let Some(task) = poller.poll().await {
        match task {
            DatabaseCollectionCursorsTask::NewCollection(task) => state.new_collection(task),
            DatabaseCollectionCursorsTask::DropCollection(task) => state.drop_collection(task),
            DatabaseCollectionCursorsTask::Query(task) => {
                state.handle_specific(|collection| &mut collection.query_cursors, task)
            }
            DatabaseCollectionCursorsTask::Diff(task) => {
                state.handle_specific(|collection| &mut collection.diff_cursors, task)
            }
        }
    }
}

impl CursorsThreadState {
    fn handle_specific<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: DatabaseCollectionSpecificCursorsTask<T>,
    ) {
        match task {
            DatabaseCollectionSpecificCursorsTask::AddQueryCursor(task) => {
                self.add_query_cursor(get_cursors, task)
            }
            DatabaseCollectionSpecificCursorsTask::GetQueryCursorByPublicId(task) => {
                self.get_query_cursor_by_public_id(get_cursors, task)
            }
            DatabaseCollectionSpecificCursorsTask::AddQueryCursorContinuation(task) => {
                self.add_query_cursor_continuation(get_cursors, task)
            }
            DatabaseCollectionSpecificCursorsTask::FinishQueryCursor(task) => {
                self.finish_query_cursor(get_cursors, task)
            }
            DatabaseCollectionSpecificCursorsTask::FullyFinishQueryCursor(task) => {
                self.fully_finish_query_cursor(get_cursors, task)
            }
            DatabaseCollectionSpecificCursorsTask::AbortQueryCursor(task) => {
                self.abort_query_cursor(get_cursors, task)
            }
            #[cfg(test)]
            DatabaseCollectionSpecificCursorsTask::GetCollectionQueryCursorsCount(task) => {
                self.collection_query_cursors_count(get_cursors, task)
            }
        }
    }

    fn new_collection(&mut self, task: NewCollectionTask) {
        let NewCollectionTask { sender } = task;

        let id = self
            .collections
            .insert(|id| InnerCursorsCollection::new(&self.config, id));

        if let Err(_) = sender.send(id) {
            self.collections.delete(&id);
        }
    }

    fn drop_collection(&mut self, task: DropCollectionTask) {
        let DropCollectionTask { collection_id } = task;

        self.collections.delete(&collection_id);
    }

    fn add_query_cursor<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: AddCursorTask<T>,
    ) {
        let AddCursorTask {
            collection_id,
            data,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(CursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let public_id = get_cursors(collection).add_cursor(data);

        sender.send(Ok(public_id)).unwrap_or(());
    }

    fn get_query_cursor_by_public_id<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: GetCursorByPublicIdTask<T>,
    ) {
        let GetCursorByPublicIdTask {
            collection_id,
            public_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(None).unwrap_or(());
            return;
        };

        let Some(cursor) = get_cursors(collection).cursor_by_public_id(public_id) else {
            sender.send(None).unwrap_or(());
            return;
        };

        sender.send(Some(cursor)).unwrap_or(());
    }

    fn add_query_cursor_continuation<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: AddCursorContinuationTask<T>,
    ) {
        let AddCursorContinuationTask {
            collection_id,
            inner_id,
            is_current,
            data,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(CursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let result = get_cursors(collection).add_cursor_continuation(&inner_id, data, is_current);

        sender.send(result).unwrap_or(());
    }

    fn finish_query_cursor<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: FinishCursorTask<T>,
    ) {
        let FinishCursorTask {
            collection_id,
            inner_id,
            is_current,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(CursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let result = get_cursors(collection).finish_cursor(&inner_id, is_current);

        sender.send(result).unwrap_or(());
    }

    fn fully_finish_query_cursor<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: FullyFinishCursorTask<T>,
    ) {
        let FullyFinishCursorTask {
            collection_id,
            inner_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(CursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let result = get_cursors(collection).fully_finish_cursor(&inner_id);

        sender.send(result).unwrap_or(());
    }

    fn abort_query_cursor<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: AbortCursorTask<T>,
    ) {
        let AbortCursorTask {
            cursor_type: _,
            collection_id,
            public_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(CursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let cursors = get_cursors(collection);

        let Some((inner_id, _)) = cursors.cursor_by_public_id(public_id) else {
            sender.send(Err(CursorError::NoSuchCursor)).unwrap_or(());
            return;
        };

        let result = cursors.abort_cursor(&inner_id);

        sender.send(result).unwrap_or(());
    }

    #[cfg(test)]
    fn collection_query_cursors_count<
        T: CursorType,
        F: Fn(&mut InnerCursorsCollection) -> &mut InnerCursors<T>,
    >(
        &mut self,
        get_cursors: F,
        task: GetCollectionCursorsCountTask<T>,
    ) {
        let GetCollectionCursorsCountTask {
            cursor_type: _,
            collection_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(CursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let count = get_cursors(collection).query_cursors_count();

        sender.send(Ok(count)).unwrap_or(());
    }
}
