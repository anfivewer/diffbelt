use crate::common::OwnedGenerationId;
use crate::database::generations::collection::{
    InnerGenerationsCollection, InnerGenerationsCollectionId, NextGenerationLocked,
    NextGenerationScheduleAction,
};
use std::sync::Arc;

use crate::database::DatabaseInner;
use crate::messages::generations::{
    AbortManualGenerationTask, CommitManualGenerationError, CommitManualGenerationTask,
    DatabaseCollectionGenerationsTask, DropCollectionGenerationsTask, LockManualGenerationIdError,
    LockNextGenerationIdTask, LockNextGenerationIdTaskResponse, NewCollectionGenerationsTask,
    NewCollectionGenerationsTaskResponse, StartManualGenerationIdError,
    StartManualGenerationIdTask,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::indexed_container::IndexedContainer;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

struct GenerationsThreadState {
    database: Arc<DatabaseInner>,
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

    let DatabaseCollectionGenerationsTask::Init(database) = task else {
        panic!("database/generations/thread first task is not init");
    };

    let (sender, mut receiver) = mpsc::channel::<ThreadTask>(8);

    let mut state = GenerationsThreadState {
        database,
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
                DatabaseCollectionGenerationsTask::AbortManualGeneration(task) => {
                    state.abort_manual_generation(task);
                }
                DatabaseCollectionGenerationsTask::CommitManualGeneration(task) => {
                    state.commit_manual_generation(task);
                }
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
            name,
            is_manual,
            generation_id,
            next_generation_id,
            db,
            is_deleted,
            sender,
        } = task;

        let id = self.collections.insert(|inner_id| {
            InnerGenerationsCollection::new(
                inner_id,
                name,
                is_manual,
                db,
                generation_id.clone(),
                next_generation_id,
                is_deleted,
            )
        });

        let item = self.collections.get(&id).unwrap();

        if is_manual {
            if let Err(_) = sender.send(Ok(NewCollectionGenerationsTaskResponse {
                collection_id: id,
                generation_pair_receiver: item.generation_pair_receiver.clone(),
            })) {
                self.collections.delete(&id);
            }

            return;
        }

        let generation_pair_receiver = item.generation_pair_receiver.clone();

        let thread_task_sender = self.sender.clone();
        let is_need_to_schedule_generation =
            item.is_need_to_schedule_generation(generation_id.incremented());

        tokio::spawn(async move {
            let need_to_schedule = match is_need_to_schedule_generation.await {
                Ok(need_to_schedule) => need_to_schedule,
                Err(err) => {
                    sender.send(Err(err)).unwrap_or(());
                    thread_task_sender
                        .send(ThreadTask::External(
                            DatabaseCollectionGenerationsTask::DropCollection(
                                DropCollectionGenerationsTask {
                                    collection_id: id,
                                    sender: None,
                                },
                            ),
                        ))
                        .await
                        .unwrap_or(());
                    return;
                }
            };

            let response = Ok(NewCollectionGenerationsTaskResponse {
                collection_id: id,
                generation_pair_receiver,
            });
            let mut need_delete = false;

            match need_to_schedule {
                NextGenerationScheduleAction::NeedSchedule => {
                    thread_task_sender
                        .send(ThreadTask::ScheduleNextGeneration {
                            collection_id: id,
                            expected_generation_id: generation_id,
                        })
                        .await
                        .unwrap_or(());

                    if let Err(_) = sender.send(response) {
                        need_delete = true;
                    }
                }
                NextGenerationScheduleAction::NoNeedSchedule => {
                    if let Err(_) = sender.send(response) {
                        need_delete = true;
                    }
                }
            }

            if need_delete {
                thread_task_sender
                    .send(ThreadTask::External(
                        DatabaseCollectionGenerationsTask::DropCollection(
                            DropCollectionGenerationsTask {
                                collection_id: id,
                                sender: None,
                            },
                        ),
                    ))
                    .await
                    .unwrap_or(());
            }
        });
    }

    fn drop_collection(&mut self, task: DropCollectionGenerationsTask) {
        let DropCollectionGenerationsTask {
            collection_id,
            sender,
        } = task;

        self.collections.delete(&collection_id);

        sender.map(|sender| sender.send(()).unwrap_or(()));
    }

    fn start_manual_generation(&mut self, task: StartManualGenerationIdTask) {
        let StartManualGenerationIdTask {
            collection_id,
            sender,
            next_generation_id,
            abort_outdated,
        } = task;

        let Some(item) = self.collections.get_mut(&collection_id) else {
            sender
                .send(Err(StartManualGenerationIdError::NoSuchCollection))
                .unwrap_or(());
            return;
        };

        let fut = item.start_manual_generation(next_generation_id, abort_outdated);

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
            next_generation_id,
            is_phantom,
        } = task;

        let Some(item) = self.collections.get_mut(&collection_id) else {
            sender
                .send(Err(LockManualGenerationIdError::NoSuchCollection))
                .unwrap_or(());
            return;
        };

        let is_manual = item.is_manual;
        let locked = item.lock_next_generation(next_generation_id, is_phantom);

        let thread_task_sender = self.sender.clone();

        tokio::spawn(async move {
            let locked = match locked.await {
                Ok(locked) => locked,
                Err(err) => {
                    sender.send(Err(err)).unwrap_or(());
                    return;
                }
            };

            let NextGenerationLocked {
                next_generation_id,
                lock,
                unlock_receiver,
            } = locked;

            let generation_id = lock.generation_id().to_owned();

            sender
                .send(Ok(LockNextGenerationIdTaskResponse {
                    next_generation_id,
                    lock,
                }))
                .unwrap_or(());

            if is_manual || is_phantom {
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

        let NextGenerationScheduleAction::NeedSchedule =
            item.schedule_next_generation(expected_generation_id)
        else {
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

    fn abort_manual_generation(&mut self, task: AbortManualGenerationTask) {
        let AbortManualGenerationTask {
            collection_id,
            sender,
            generation_id,
        } = task;

        let Some(item) = self.collections.get_mut(&collection_id) else {
            sender
                .send(Err(CommitManualGenerationError::NoSuchCollection))
                .unwrap_or(());
            return;
        };

        let aborting = item.abort_manual_generation(generation_id);

        tokio::spawn(async move {
            let result = aborting.await;

            sender.send(result).unwrap_or(());
        });
    }

    fn commit_manual_generation(&mut self, task: CommitManualGenerationTask) {
        let CommitManualGenerationTask {
            collection_id,
            sender,
            generation_id,
            update_readers,
        } = task;

        let Some(item) = self.collections.get_mut(&collection_id) else {
            sender
                .send(Err(CommitManualGenerationError::NoSuchCollection))
                .unwrap_or(());
            return;
        };

        let committing =
            item.commit_manual_generation(self.database.clone(), generation_id, update_readers);

        tokio::spawn(async move {
            let result = committing.await;

            sender.send(result).unwrap_or(());
        });
    }
}
