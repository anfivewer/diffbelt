use crate::common::OwnedGenerationId;
use crate::messages::readers::{
    CollectionNameReaderName, DatabaseCollectionReadersTask, DeleteReaderTask,
    GetReadersPointingToCollectionTask, ReaderNewCollectionTask, ReaderNewCollectionTaskResponse,
    UpdateReaderTask, UpdateReadersTask,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::hashmap::{ArcStringPair, ArcStringPairRef};

use hashbrown::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::watch;

type CollectionName = Arc<str>;
type ReaderName = Arc<str>;

struct Reader {
    owner_collection_name: CollectionName,
    to_collection_name: CollectionName,
    reader_name: ReaderName,
    generation_id: OwnedGenerationId,
}

struct CollectionState {
    minimum_generation_id_sender: watch::Sender<OwnedGenerationId>,
    minimum_generation_id_receiver: watch::Receiver<OwnedGenerationId>,
    // (owner_collection_name, reader_name)
    readers_pointing_to_collection: HashMap<ArcStringPair, Arc<Reader>>,
}

struct ReadersState {
    // (owner_collection_name, reader_name)
    all_readers: HashMap<ArcStringPair, Arc<Reader>>,
    collections: HashMap<CollectionName, CollectionState>,
    changed_readers_pointing_to_collections: HashSet<CollectionName>,
}

pub async fn run(_: (), mut poller: TaskPoller<DatabaseCollectionReadersTask>) {
    let task = poller.poll().await;
    let Some(task) = task else {
        return;
    };

    let DatabaseCollectionReadersTask::Init(_database) = task else {
        panic!("database/readers/thread first task is not init");
    };

    let mut state = ReadersState {
        all_readers: HashMap::new(),
        collections: HashMap::new(),
        changed_readers_pointing_to_collections: HashSet::new(),
    };

    let mut is_init_finished = false;

    while let Some(task) = poller.poll().await {
        match task {
            DatabaseCollectionReadersTask::NewCollection(task) => {
                state.new_collection(task);
            }
            DatabaseCollectionReadersTask::UpdateReader(task) => {
                state.update_reader(task);
            }
            DatabaseCollectionReadersTask::UpdateReaders(task) => {
                let UpdateReadersTask { updates } = task;

                for update in updates {
                    state.update_reader(update);
                }
            }
            DatabaseCollectionReadersTask::DeleteReader(task) => {
                state.delete_reader(task);
            }
            DatabaseCollectionReadersTask::GetReadersPointingToCollectionExceptThisOne(task) => {
                state.get_readers_pointing_to_collection_except_this_one(task);
            }
            DatabaseCollectionReadersTask::Finish => {
                return;
            }
            DatabaseCollectionReadersTask::InitFinish => {
                is_init_finished = true;
            }
            DatabaseCollectionReadersTask::Init(_) => {}
        }

        if is_init_finished {
            if !state.changed_readers_pointing_to_collections.is_empty() {
                state.check_for_gc();
            }
        }
    }
}

impl ReadersState {
    fn new_collection(&mut self, task: ReaderNewCollectionTask) {
        let ReaderNewCollectionTask {
            collection_name,
            sender,
        } = task;

        let collection = self.collections.get(&collection_name);

        let minimum_generation_id = match collection {
            None => {
                let (sender, receiver) = watch::channel(OwnedGenerationId::empty());

                self.collections.insert(
                    collection_name,
                    CollectionState {
                        minimum_generation_id_sender: sender,
                        minimum_generation_id_receiver: receiver.clone(),
                        readers_pointing_to_collection: Default::default(),
                    },
                );

                receiver
            }
            Some(collection) => collection.minimum_generation_id_receiver.clone(),
        };

        sender
            .send(ReaderNewCollectionTaskResponse {
                minimum_generation_id,
            })
            .unwrap_or(());
    }

    fn update_reader(&mut self, update: UpdateReaderTask) {
        let UpdateReaderTask {
            owner_collection_name,
            to_collection_name,
            reader_name,
            generation_id,
        } = update;

        let collection_name_reader_name_key =
            ArcStringPairRef(owner_collection_name.as_ref(), reader_name.as_ref());

        let existing_reader = self.all_readers.get(&collection_name_reader_name_key);

        let (reader_name, to_collection_name) = match existing_reader {
            Some(existing_reader) => {
                let is_to_collection_changed = if let Some(to_collection_name) = &to_collection_name
                {
                    to_collection_name.as_ref() != existing_reader.to_collection_name.as_ref()
                } else {
                    false
                };

                let is_generation_id_changed = generation_id != existing_reader.generation_id;

                if !is_generation_id_changed && !is_to_collection_changed {
                    return;
                }

                if is_to_collection_changed {
                    self.changed_readers_pointing_to_collections
                        .insert(existing_reader.to_collection_name.clone());

                    let collection = self
                        .collections
                        .get_mut(existing_reader.to_collection_name.as_ref());
                    if let Some(collection) = collection {
                        collection
                            .readers_pointing_to_collection
                            .remove(&collection_name_reader_name_key);
                    }
                }

                (
                    existing_reader.reader_name.clone(),
                    to_collection_name.or_else(|| Some(existing_reader.to_collection_name.clone())),
                )
            }
            None => (Arc::from(reader_name), to_collection_name),
        };

        let to_collection_name =
            to_collection_name.unwrap_or_else(|| owner_collection_name.clone());

        self.changed_readers_pointing_to_collections
            .insert(to_collection_name.clone());

        let reader = Arc::new(Reader {
            owner_collection_name: owner_collection_name.clone(),
            to_collection_name: to_collection_name.clone(),
            reader_name: reader_name.clone(),
            generation_id,
        });

        let collection_name_reader_name_key = ArcStringPair(owner_collection_name, reader_name);

        self.all_readers
            .insert(collection_name_reader_name_key.clone(), reader.clone());

        let collection = self.collections.get_mut(to_collection_name.as_ref());

        if let Some(collection) = collection {
            collection
                .readers_pointing_to_collection
                .insert(collection_name_reader_name_key, reader);
        } else {
            let generation_id = reader.generation_id.clone();

            let mut readers_map = HashMap::new();
            readers_map.insert(collection_name_reader_name_key, reader);

            let (sender, receiver) = watch::channel(generation_id);

            self.collections.insert(
                to_collection_name,
                CollectionState {
                    minimum_generation_id_sender: sender,
                    minimum_generation_id_receiver: receiver,
                    readers_pointing_to_collection: readers_map,
                },
            );
        }
    }

    fn delete_reader(&mut self, task: DeleteReaderTask) {
        let DeleteReaderTask {
            owner_collection_name,
            reader_name,
        } = task;

        let collection_name_reader_name_key = ArcStringPair(owner_collection_name, reader_name);

        let reader = self.all_readers.get(&collection_name_reader_name_key);
        let Some(reader) = reader else {
            return;
        };

        self.changed_readers_pointing_to_collections
            .insert(reader.to_collection_name.clone());

        let collection = self.collections.get_mut(reader.to_collection_name.as_ref());
        let Some(collection) = collection else {
            return;
        };

        collection
            .readers_pointing_to_collection
            .remove(&collection_name_reader_name_key);
    }

    fn get_readers_pointing_to_collection_except_this_one(
        &mut self,
        task: GetReadersPointingToCollectionTask,
    ) {
        let GetReadersPointingToCollectionTask {
            collection_name,
            sender,
        } = task;

        let collection = self.collections.get(&collection_name);
        let Some(collection) = collection else {
            sender.send(Vec::with_capacity(0)).unwrap_or(());
            return;
        };

        let readers = &collection.readers_pointing_to_collection;

        let mut result = Vec::with_capacity(readers.len());

        for reader in readers.values() {
            if reader.owner_collection_name == collection_name {
                continue;
            }

            result.push(CollectionNameReaderName {
                owner_collection_name: reader.owner_collection_name.clone(),
                reader_name: reader.reader_name.clone(),
            });
        }

        sender.send(result).unwrap_or(());
    }

    fn check_for_gc(&mut self) {
        let iter = self.changed_readers_pointing_to_collections.drain();

        for collection_name in iter {
            let Some(collection) = self.collections.get(&collection_name) else {
                continue;
            };

            let mut iter = collection.readers_pointing_to_collection.values();

            let mut minimum_generation_id = {
                let Some(first_reader) = iter.next() else {
                    // Collection has no readers pointing to it?
                    continue;
                };

                &first_reader.generation_id
            };

            for reader in iter {
                let generation_id = &reader.generation_id;

                if generation_id < minimum_generation_id {
                    minimum_generation_id = generation_id;
                }
            }

            collection
                .minimum_generation_id_sender
                .send(minimum_generation_id.clone())
                .unwrap_or(());
        }
    }
}
