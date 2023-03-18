use crate::common::OwnedGenerationId;
use crate::messages::readers::{
    CollectionNameReaderName, DatabaseCollectionReadersTask, DeleteReaderTask,
    GetReadersPointingToCollectionTask, UpdateReaderTask,
};
use crate::util::async_task_thread::TaskPoller;
use crate::util::hashmap::{ArcStringPair, ArcStringPairRef};

use hashbrown::HashMap;
use std::sync::Arc;

type CollectionName = Arc<str>;
type ReaderName = Arc<str>;

struct Reader {
    pub owner_collection_name: CollectionName,
    pub to_collection_name: CollectionName,
    pub reader_name: ReaderName,
    pub generation_id: Arc<OwnedGenerationId>,
}

struct ReadersState {
    // (owner_collection_name, reader_name)
    all_readers: HashMap<ArcStringPair, Arc<Reader>>,
    // to_collection_name => (owner_collection_name, reader_name)
    pointing_to_collection: HashMap<CollectionName, HashMap<ArcStringPair, Arc<Reader>>>,
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
        pointing_to_collection: HashMap::new(),
    };

    while let Some(task) = poller.poll().await {
        match task {
            DatabaseCollectionReadersTask::UpdateReader(task) => {
                state.update_reader(task);
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
            _ => {}
        }
    }
}

impl ReadersState {
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

                let is_generation_id_changed =
                    Arc::as_ref(&generation_id) != Arc::as_ref(&existing_reader.generation_id);

                if !is_generation_id_changed && !is_to_collection_changed {
                    return;
                }

                if is_to_collection_changed {
                    let readers_map = self
                        .pointing_to_collection
                        .get_mut(existing_reader.to_collection_name.as_ref());
                    if let Some(readers_map) = readers_map {
                        readers_map.remove(&collection_name_reader_name_key);
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

        let reader = Arc::new(Reader {
            owner_collection_name: owner_collection_name.clone(),
            to_collection_name: to_collection_name.clone(),
            reader_name: reader_name.clone(),
            generation_id,
        });

        let collection_name_reader_name_key = ArcStringPair(owner_collection_name, reader_name);

        self.all_readers
            .insert(collection_name_reader_name_key.clone(), reader.clone());

        let readers_map = self
            .pointing_to_collection
            .get_mut(to_collection_name.as_ref());

        if let Some(readers_map) = readers_map {
            readers_map.insert(collection_name_reader_name_key, reader);
        } else {
            let mut readers_map = HashMap::new();
            readers_map.insert(collection_name_reader_name_key, reader);

            self.pointing_to_collection
                .insert(to_collection_name, readers_map);
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

        let readers = self
            .pointing_to_collection
            .get_mut(reader.to_collection_name.as_ref());
        let Some(readers) = readers else {
            return;
        };

        readers.remove(&collection_name_reader_name_key);
    }

    fn get_readers_pointing_to_collection_except_this_one(
        &mut self,
        task: GetReadersPointingToCollectionTask,
    ) {
        let GetReadersPointingToCollectionTask {
            collection_name,
            sender,
        } = task;

        let readers = self.pointing_to_collection.get(&collection_name);
        let Some(readers) = readers else {
            sender.send(Vec::with_capacity(0)).unwrap_or(());
            return;
        };

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
}
