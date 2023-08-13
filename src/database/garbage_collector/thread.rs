use crate::common::collection::CollectionName;
use crate::database::config::DatabaseConfig;
use crate::database::garbage_collector::collection::GarbageCollectorCollection;
use crate::messages::garbage_collector::{
    CleanupGenerationsLessThanTask, DatabaseGarbageCollectorTask, DropCollectionTask,
    GarbageCollectorCommonError, NewCollectionTask, NewCollectionTaskResponse,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::auto_sender_on_drop::AutoSenderOnDrop;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::spawn_local;

struct GarbageCollectorState {
    config: Arc<DatabaseConfig>,
    task_sender: mpsc::Sender<DatabaseGarbageCollectorTask>,
    counter: Cell<usize>,
    collections: RefCell<HashMap<CollectionName, Rc<GarbageCollectorCollection>>>,
}

pub async fn run(_: (), mut poller: TaskPoller<DatabaseGarbageCollectorTask>) {
    let task = poller.poll().await;
    let Some(task) = task else {
        return;
    };

    let DatabaseGarbageCollectorTask::Init(database) = task else {
        panic!("database/garbage_collector/thread first task is not init");
    };

    let config = database.config.clone();

    drop(database);

    let state = Rc::new(GarbageCollectorState {
        config,
        task_sender: poller.task_sender.clone(),
        counter: Cell::new(0),
        collections: RefCell::new(HashMap::new()),
    });

    while let Some(task) = poller.poll().await {
        match task {
            DatabaseGarbageCollectorTask::NewCollection(task) => {
                state.clone().new_collection(task);
            }
            DatabaseGarbageCollectorTask::DropCollection(task) => {
                state.clone().drop_collection(task);
            }
            DatabaseGarbageCollectorTask::CleanupGenerationsLessThan(task) => {
                state.clone().cleanup_generations_less_than(task);
            }
            DatabaseGarbageCollectorTask::Init(_) => {}
        }
    }
}

impl GarbageCollectorState {
    fn new_collection(self: Rc<Self>, task: NewCollectionTask) {
        let NewCollectionTask {
            collection_name,
            raw_db,
            is_deleted,
            sender,
        } = task;

        let already_exists = { self.collections.borrow().contains_key(&collection_name) };

        if already_exists {
            sender
                .send(Err(
                    GarbageCollectorCommonError::SuchCollectionAlreadyExists,
                ))
                .unwrap_or(());
            return;
        }

        let id = self.counter.replace(self.counter.get() + 1);

        let collection = GarbageCollectorCollection::new(id, raw_db, is_deleted);

        {
            self.collections
                .borrow_mut()
                .insert(collection_name.clone(), Rc::new(collection));
        }

        let (drop_handle, drop_receiver) = AutoSenderOnDrop::new(());

        sender
            .send(Ok(NewCollectionTaskResponse { id, drop_handle }))
            .unwrap_or(());

        spawn_local(async move {
            drop_receiver.await;

            let mut collections = self.collections.borrow_mut();

            let item = collections.remove(&collection_name);

            let is_wrong_one = item
                .as_ref()
                .map(|collection| collection.id != id)
                .unwrap_or(false);

            if is_wrong_one {
                collections.insert(collection_name, item.unwrap());
            }
        });
    }

    fn drop_collection(self: Rc<Self>, task: DropCollectionTask) {
        let DropCollectionTask {
            collection_name,
            id,
            sender,
        } = task;

        let mut collections = self.collections.borrow_mut();

        let item = collections.remove(&collection_name);

        let is_wrong_one = item
            .as_ref()
            .map(|collection| collection.id != id)
            .unwrap_or(false);

        if is_wrong_one {
            collections.insert(collection_name, item.unwrap());
        }

        if let Some(sender) = sender {
            sender.send(()).unwrap_or(());
        }
    }

    fn cleanup_generations_less_than(self: Rc<Self>, task: CleanupGenerationsLessThanTask) {
        let CleanupGenerationsLessThanTask {
            collection_name,
            generation_id_less_than,
        } = task;

        let collections = self.collections.borrow();
        let item = collections.get(collection_name.as_ref());

        let Some(item) = item else {
            return;
        };

        let item = item.clone();

        drop(collections);

        item.cleanup_generations_less_than(&self.config, generation_id_less_than);
    }
}
