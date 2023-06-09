use crate::database::generations::collection::{
    InnerGenerationsCollection, InnerGenerationsCollectionId,
};
use crate::database::generations::next_generation_lock::{
    NextGenerationIdLock, NextGenerationIdLockWithSender,
};
use crate::messages::generations::{
    DatabaseCollectionGenerationsTask, DropCollectionGenerationsTask, LockNextGenerationIdTask,
    LockNextGenerationIdTaskResponse, NewCollectionGenerationsTask,
    NewCollectionGenerationsTaskResponse,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::indexed_container::IndexedContainer;
use tokio::sync::{mpsc, oneshot};

struct GenerationsThreadState {
    collections: IndexedContainer<InnerGenerationsCollection>,
}

enum ThreadTask {
    External(DatabaseCollectionGenerationsTask),
    UnlockNextGeneration {
        collection_id: InnerGenerationsCollectionId,
        lock: NextGenerationIdLock,
    },
}

pub async fn run(_: (), mut poller: TaskPoller<DatabaseCollectionGenerationsTask>) {
    let task = poller.poll().await;
    let Some(task) = task else {
        return;
    };

    let DatabaseCollectionGenerationsTask::Init(_database) = task else {
        panic!("database/generations/thread first task is not init");
    };

    let (sender, mut receiver) = mpsc::channel::<ThreadTask>(8);

    let mut state = GenerationsThreadState {
        collections: IndexedContainer::new(),
    };

    while let Some(task) = poll_task(&mut poller, &mut receiver).await {
        match task {
            ThreadTask::External(task) => match task {
                DatabaseCollectionGenerationsTask::NewCollection(task) => {
                    state.new_collection(task);
                }
                DatabaseCollectionGenerationsTask::DropCollection(task) => {
                    state.drop_collection(task);
                }
                DatabaseCollectionGenerationsTask::LockNextGenerationId(task) => {
                    state.lock_next_generation(task, sender.clone());
                }
                _ => {}
            },
            ThreadTask::UnlockNextGeneration {
                collection_id,
                lock,
            } => {
                state.unlock_next_generation(collection_id, lock);
            }
        }
    }
}

async fn poll_task(
    poller: &mut TaskPoller<DatabaseCollectionGenerationsTask>,
    receiver: &mut mpsc::Receiver<ThreadTask>,
) -> Option<ThreadTask> {
    tokio::select! {
        maybe_task = poller.poll() => {
            return maybe_task.map(|task| ThreadTask::External(task));
        },
        maybe_task = receiver.recv() => {
            return maybe_task;
        },
    }
}

impl GenerationsThreadState {
    fn new_collection(&mut self, task: NewCollectionGenerationsTask) {
        let NewCollectionGenerationsTask {
            generation_id,
            next_generation_id,
            sender,
        } = task;

        let id = self.collections.insert(move |inner_id| {
            InnerGenerationsCollection::new(inner_id, generation_id, next_generation_id)
        });

        let item = self.collections.get(&id).unwrap();

        if let Err(_) = sender.send(NewCollectionGenerationsTaskResponse {
            collection_id: id,
            generation_id_receiver: item.generation_id_receiver.clone(),
        }) {
            self.collections.delete(&id);
        }
    }

    fn drop_collection(&mut self, task: DropCollectionGenerationsTask) {
        let DropCollectionGenerationsTask { collection_id } = task;

        self.collections.delete(&collection_id);
    }

    fn lock_next_generation(
        &mut self,
        task: LockNextGenerationIdTask,
        thread_task_sender: mpsc::Sender<ThreadTask>,
    ) {
        let LockNextGenerationIdTask {
            collection_id,
            sender,
        } = task;

        let Some(item) = self.collections.get_mut(&collection_id) else {
            return;
        };

        let lock = item.lock_next_generation();

        let (lock_sender, receiver) = oneshot::channel();

        let lock_with_sender = NextGenerationIdLockWithSender {
            sender: Some(lock_sender),
        };

        tokio::spawn(async move {
            let Ok(_) = receiver.await else {
                return;
            };

            thread_task_sender
                .send(ThreadTask::UnlockNextGeneration {
                    collection_id,
                    lock,
                })
                .await
                .unwrap_or(());
        });

        sender
            .send(LockNextGenerationIdTaskResponse {
                generation_id: item.generation_id.clone(),
                next_generation_id: item.next_generation_id.clone(),
                lock: lock_with_sender,
            })
            .unwrap_or(());
    }

    fn unlock_next_generation(
        &mut self,
        collection_id: InnerGenerationsCollectionId,
        lock: NextGenerationIdLock,
    ) {
        let Some(item) = self.collections.get_mut(&collection_id) else {
            return;
        };

        item.unlock_next_generation(lock);
    }
}
