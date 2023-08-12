use crate::common::OwnedGenerationId;
use crate::database::generations::collection::{
    InnerGenerationsCollection, InnerGenerationsCollectionId, NextGenerationLocked,
    NextGenerationScheduleAction,
};

use crate::messages::generations::{
    DatabaseCollectionGenerationsTask, DropCollectionGenerationsTask, LockNextGenerationIdTask,
    LockNextGenerationIdTaskResponse, NewCollectionGenerationsTask,
    NewCollectionGenerationsTaskResponse, StartManualGenerationIdError,
    StartManualGenerationIdTask,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::indexed_container::IndexedContainer;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

struct GenerationsThreadState {
    sender: mpsc::Sender<ThreadTask>,
    collections: IndexedContainer<InnerGenerationsCollection>,
}

enum ThreadTask {
    External(DatabaseCollectionGenerationsTask),
    ScheduleNextGeneration {
        collection_id: InnerGenerationsCollectionId,
        expected_generation_id: OwnedGenerationId,
    },
    CommitNonManualCollectionGeneration {
        collection_id: InnerGenerationsCollectionId,
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
        sender,
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
                    state.lock_next_generation(task);
                }
                DatabaseCollectionGenerationsTask::StartManualGenerationId(task) => {
                    state.start_manual_generation(task);
                }
                DatabaseCollectionGenerationsTask::LockManualGenerationId(_) => {}
                DatabaseCollectionGenerationsTask::CommitManualGeneration(_) => {}
                DatabaseCollectionGenerationsTask::Init(_) => {}
            },
            ThreadTask::ScheduleNextGeneration {
                collection_id,
                expected_generation_id: expected_next_generation_id,
            } => {
                state.schedule_next_generation(collection_id, expected_next_generation_id);
            }
            ThreadTask::CommitNonManualCollectionGeneration { collection_id } => {
                state.commit_non_manual_collection_generation(collection_id);
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
            is_manual,
            generation_id,
            next_generation_id,
            db,
            sender,
        } = task;

        let id = self.collections.insert(move |inner_id| {
            InnerGenerationsCollection::new(
                inner_id,
                is_manual,
                db,
                generation_id,
                next_generation_id,
            )
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

    fn start_manual_generation(&mut self, task: StartManualGenerationIdTask) {
        let StartManualGenerationIdTask {
            collection_id,
            sender,
            next_generation_id,
        } = task;

        let Some(item) = self.collections.get_mut(&collection_id) else {
            return;
        };

        let fut = item.start_manual_generation(next_generation_id);

        tokio::spawn(async move {
            let result = fut.await;

            match result {
                Ok(()) => {
                    sender.send(Ok(())).unwrap_or(());
                }
                Err(error) => {
                    sender.send(Err(error)).unwrap_or(());
                }
            }
        });
    }

    fn lock_next_generation(&mut self, task: LockNextGenerationIdTask) {
        let LockNextGenerationIdTask {
            collection_id,
            sender,
        } = task;

        let Some(item) = self.collections.get_mut(&collection_id) else {
            return;
        };

        let is_manual = item.is_manual;

        let locked = item.lock_next_generation();

        let thread_task_sender = self.sender.clone();

        tokio::spawn(async move {
            let locked = locked.await;

            let NextGenerationLocked {
                generation_id,
                next_generation_id,
                lock,
                unlock_receiver,
            } = locked;

            sender
                .send(LockNextGenerationIdTaskResponse {
                    generation_id: generation_id.clone(),
                    next_generation_id,
                    lock,
                })
                .unwrap_or(());

            if is_manual {
                return;
            }

            let Ok(lock_data) = unlock_receiver.await else {
                return;
            };

            if !lock_data.need_schedule_next_generation {
                return;
            }

            thread_task_sender
                .send(ThreadTask::ScheduleNextGeneration {
                    collection_id,
                    expected_generation_id: generation_id,
                })
                .await
                .unwrap_or(());
        });
    }

    fn schedule_next_generation(
        &mut self,
        collection_id: InnerGenerationsCollectionId,
        expected_generation_id: OwnedGenerationId,
    ) {
        let Some(item) = self.collections.get_mut(&collection_id) else {
            return;
        };

        let NextGenerationScheduleAction::NeedSchedule = item.schedule_next_generation(expected_generation_id) else {
            return;
        };

        let thread_task_sender = self.sender.clone();

        tokio::spawn(async move {
            // TODO: move to the config, or better to collection settings
            sleep(Duration::from_millis(50)).await;

            thread_task_sender
                .send(ThreadTask::CommitNonManualCollectionGeneration { collection_id })
                .await
                .unwrap_or(());
        });
    }

    fn commit_non_manual_collection_generation(
        &mut self,
        collection_id: InnerGenerationsCollectionId,
    ) {
        let Some(item) = self.collections.get_mut(&collection_id) else {
            return;
        };

        item.commit_next_generation();
    }
}
