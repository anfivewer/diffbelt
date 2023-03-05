use crate::database::cursors::collection::InnerCursorsCollection;
use crate::messages::cursors::DatabaseCollectionCursorsTask;
use crate::util::async_task_thread::TaskPoller;
use crate::util::indexed_container::IndexedContainer;

struct CursorsThreadState {
    collections: IndexedContainer<InnerCursorsCollection>,
}

pub async fn run(_: (), mut poller: TaskPoller<DatabaseCollectionCursorsTask>) {
    let state = CursorsThreadState {
        collections: IndexedContainer::new(),
    };

    while let Some(_task) = poller.poll().await {
        //
    }
}
