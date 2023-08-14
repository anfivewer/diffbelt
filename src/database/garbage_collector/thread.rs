use crate::common::collection::CollectionName;
use crate::database::config::DatabaseConfig;
use crate::database::garbage_collector::collection::GarbageCollectorCollection;
use crate::messages::garbage_collector::{
    DatabaseGarbageCollectorTask, GarbageCollectorCommonError, GarbageCollectorDropCollectionTask,
    GarbageCollectorNewCollectionTask, NewCollectionTaskResponse,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::auto_sender_on_drop::AutoSenderOnDrop;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::task::spawn_local;

struct GarbageCollectorState {
    config: Arc<DatabaseConfig>,
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
            DatabaseGarbageCollectorTask::Init(_) => {}
        }
    }
}

impl GarbageCollectorState {
    fn new_collection(self: Rc<Self>, task: GarbageCollectorNewCollectionTask) {
        let GarbageCollectorNewCollectionTask {
            collection_name,
            raw_db,
            is_deleted,
            minimum_generation_id,
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

        let collection = Rc::new(GarbageCollectorCollection::new(id, raw_db, is_deleted));

        {
            self.collections
                .borrow_mut()
                .insert(collection_name.clone(), collection.clone());
        }

        let (drop_handle, drop_receiver) = AutoSenderOnDrop::new(());

        sender
            .send(Ok(NewCollectionTaskResponse { id, drop_handle }))
            .unwrap_or(());

        let (drop_sender, drop_receiver2) = oneshot::channel();

        collection.cleanup_generations_less_than(
            &self.config,
            minimum_generation_id,
            drop_receiver2,
        );

        spawn_local(async move {
            drop_receiver.await;

            drop_sender.send(()).unwrap_or(());

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

    fn drop_collection(self: Rc<Self>, task: GarbageCollectorDropCollectionTask) {
        let GarbageCollectorDropCollectionTask {
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
}
