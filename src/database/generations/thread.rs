use crate::database::generations::collection::InnerGenerationsCollection;
use crate::messages::generations::{
    DatabaseCollectionGenerationsTask, DropCollectionGenerationsTask, NewCollectionGenerationsTask,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::indexed_container::IndexedContainer;

struct GenerationsThreadState {
    collections: IndexedContainer<InnerGenerationsCollection>,
}

pub async fn run(_: (), mut poller: TaskPoller<DatabaseCollectionGenerationsTask>) {
    let task = poller.poll().await;
    let Some(task) = task else {
        return;
    };

    let DatabaseCollectionGenerationsTask::Init(_database) = task else {
        panic!("database/generations/thread first task is not init");
    };

    let mut state = GenerationsThreadState {
        collections: IndexedContainer::new(),
    };

    while let Some(task) = poller.poll().await {
        match task {
            DatabaseCollectionGenerationsTask::NewCollection(task) => state.new_collection(task),
            DatabaseCollectionGenerationsTask::DropCollection(task) => state.drop_collection(task),
            _ => {}
        }
    }
}

impl GenerationsThreadState {
    fn new_collection(&mut self, task: NewCollectionGenerationsTask) {
        let NewCollectionGenerationsTask { sender } = task;

        let id = self.collections.insert(InnerGenerationsCollection::new);

        if let Err(_) = sender.send(id) {
            self.collections.delete(&id);
        }
    }

    fn drop_collection(&mut self, task: DropCollectionGenerationsTask) {
        let DropCollectionGenerationsTask { collection_id } = task;

        self.collections.delete(&collection_id);
    }
}
