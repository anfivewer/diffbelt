use crate::database::cursors::collection::InnerCursorsCollection;
use crate::database::cursors::query::{AddQueryCursorData, QueryCursorError};

use crate::messages::cursors::{
    AddQueryCursorContinuationTask, AddQueryCursorTask, DatabaseCollectionCursorsTask,
    DropCollectionTask, FinishQueryCursorTask, FullyFinishQueryCursorTask,
    GetCollectionQueryCursorsCountTask, GetQueryCursorByPublicIdTask, NewCollectionTask,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::indexed_container::IndexedContainer;

struct CursorsThreadState {
    collections: IndexedContainer<InnerCursorsCollection>,
}

pub async fn run(_: (), mut poller: TaskPoller<DatabaseCollectionCursorsTask>) {
    let mut state = CursorsThreadState {
        collections: IndexedContainer::new(),
    };

    while let Some(task) = poller.poll().await {
        match task {
            DatabaseCollectionCursorsTask::NewCollection(task) => state.new_collection(task),
            DatabaseCollectionCursorsTask::DropCollection(task) => state.drop_collection(task),
            DatabaseCollectionCursorsTask::AddQueryCursor(task) => state.add_query_cursor(task),
            DatabaseCollectionCursorsTask::GetQueryCursorByPublicId(task) => {
                state.get_query_cursor_by_public_id(task)
            }
            DatabaseCollectionCursorsTask::AddQueryCursorContinuation(task) => {
                state.add_query_cursor_continuation(task)
            }
            DatabaseCollectionCursorsTask::FinishQueryCursor(task) => {
                state.finish_query_cursor(task)
            }
            DatabaseCollectionCursorsTask::FullyFinishQueryCursorTask(task) => {
                state.fully_finish_query_cursor(task)
            }
            #[cfg(test)]
            DatabaseCollectionCursorsTask::GetCollectionQueryCursorsCount(task) => {
                state.collection_query_cursors_count(task)
            }
        }
    }
}

impl CursorsThreadState {
    fn new_collection(&mut self, task: NewCollectionTask) {
        let NewCollectionTask { sender } = task;

        let id = self.collections.insert(InnerCursorsCollection::new);

        if let Err(_) = sender.send(id) {
            self.collections.delete(&id);
        }
    }

    fn drop_collection(&mut self, task: DropCollectionTask) {
        let DropCollectionTask { collection_id } = task;

        self.collections.delete(&collection_id);
    }

    fn add_query_cursor(&mut self, task: AddQueryCursorTask) {
        let AddQueryCursorTask {
            collection_id,
            data,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(QueryCursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let public_id = collection.query_cursors.add_cursor(data);

        sender.send(Ok(public_id)).unwrap_or(());
    }

    fn get_query_cursor_by_public_id(&mut self, task: GetQueryCursorByPublicIdTask) {
        let GetQueryCursorByPublicIdTask {
            collection_id,
            public_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(None).unwrap_or(());
            return;
        };

        let Some(cursor) = collection.query_cursors.cursor_by_public_id(public_id) else {
            sender.send(None).unwrap_or(());
            return;
        };

        sender.send(Some(cursor)).unwrap_or(());
    }

    fn add_query_cursor_continuation(&mut self, task: AddQueryCursorContinuationTask) {
        let AddQueryCursorContinuationTask {
            collection_id,
            inner_id,
            data,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(QueryCursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let result = collection
            .query_cursors
            .add_cursor_continuation(&inner_id, data);

        sender.send(result).unwrap_or(());
    }

    fn finish_query_cursor(&mut self, task: FinishQueryCursorTask) {
        let FinishQueryCursorTask {
            collection_id,
            inner_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(QueryCursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let result = collection.query_cursors.finish_cursor(&inner_id);

        sender.send(result).unwrap_or(());
    }

    fn fully_finish_query_cursor(&mut self, task: FullyFinishQueryCursorTask) {
        let FullyFinishQueryCursorTask {
            collection_id,
            inner_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(QueryCursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let result = collection.query_cursors.fully_finish_cursor(&inner_id);

        sender.send(result).unwrap_or(());
    }

    #[cfg(test)]
    fn collection_query_cursors_count(&mut self, task: GetCollectionQueryCursorsCountTask) {
        let GetCollectionQueryCursorsCountTask {
            collection_id,
            sender,
        } = task;

        let Some(collection) = self.collections.get_mut(&collection_id) else {
            sender.send(Err(QueryCursorError::NoSuchCollection)).unwrap_or(());
            return;
        };

        let count = collection.query_cursors.query_cursors_count();

        sender.send(Ok(count)).unwrap_or(());
    }
}
