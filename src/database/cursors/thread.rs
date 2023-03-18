use crate::database::cursors::collection::InnerCursorsCollection;

use crate::messages::cursors::{DatabaseCollectionCursorsTask, NewCollectionTask};
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
            DatabaseCollectionCursorsTask::DropCollection(_) => {}
            DatabaseCollectionCursorsTask::AddQueryCursor(_) => {}
            DatabaseCollectionCursorsTask::GetQueryCursorByPublicId(_) => {}
            DatabaseCollectionCursorsTask::AddQueryCursorContinuation(_) => {}
            DatabaseCollectionCursorsTask::FinishQueryCursor(_) => {}
            DatabaseCollectionCursorsTask::FullyFinishQueryCursorTask(_) => {}
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
}
