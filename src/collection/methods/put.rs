use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::util::record_flags::RecordFlags;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::collection::Collection;
use crate::common::util::is_byte_array_equal_both_opt;
use crate::common::{GenerationId, GenerationIdRef, KeyValueUpdate, PhantomId};
use crate::generation::{CollectionGenerationKeyProgress, CollectionGenerationKeyStatus};
use crate::raw_db::contains_existing_collection_record::ContainsExistingCollectionRecordOptions;
use crate::raw_db::put_collection_record::PutCollectionRecordOptions;
use std::collections::HashMap;

pub struct CollectionPutOptions {
    pub update: KeyValueUpdate,
    pub generation_id: Option<GenerationId>,
    pub phantom_id: Option<PhantomId>,
}

#[derive(Debug)]
pub struct CollectionPutOk {
    pub generation_id: GenerationId,
    // if `update.if_not_present == true`, it can be false when nothing was changed
    pub was_put: bool,
}

type CollectionPutResult = Result<CollectionPutOk, CollectionMethodError>;

impl Collection {
    pub async fn put(&self, options: CollectionPutOptions) -> CollectionPutResult {
        //// Validate request
        let update = options.update;
        let generation_id = options.generation_id;
        let phantom_id = options.phantom_id;
        let is_phantom = phantom_id.is_some();

        if is_phantom && generation_id.is_none() {
            // Phantom writes can be only to the specified generation
            return Err(CollectionMethodError::PutPhantomWithoutGenerationId);
        }

        let next_generation = self.next_generation.read().await;
        let next_generation_id = next_generation.as_ref().map(|gen| &gen.id);

        let is_generation_id_equal_to_next_one =
            is_byte_array_equal_both_opt(generation_id.as_ref(), next_generation_id);

        // Phantom puts are allowed to do everything (except to be without a specified generationId),
        // but we are already checked it above
        if !is_phantom {
            if generation_id.is_some() {
                if !is_generation_id_equal_to_next_one {
                    return Err(CollectionMethodError::OutdatedGeneration);
                }
            } else if self.is_manual {
                // we cannot put values is manual collection without specified generationId
                return Err(CollectionMethodError::CannotPutInManualCollection);
            } else if next_generation.is_none() {
                panic!("Collection::put, no next_generation in !manual collection");
            }
        }

        //// Insert
        let record_generation_id = generation_id
            .as_ref()
            .or(next_generation_id)
            .expect("Collection::put, no either generation_id or next_generation");

        let key = update.key;
        let phantom_id_or_empty = phantom_id.unwrap_or(PhantomId::empty());

        let record_key = OwnedRecordKey::new(
            key.as_ref(),
            record_generation_id.as_ref(),
            phantom_id_or_empty.as_ref(),
        )
        .or(Err(CollectionMethodError::InvalidKey))?;

        let mut resolve_put = None;

        // When `if_not_present = true`, we need to not write same record in the same time,
        // to provide correct response from this method
        // (when there is two concurrent puts, we should return `was_put = true` only in single one)
        if update.if_not_present {
            // Check hashmap of current puts or take a put lock
            let result = handle_if_not_present(
                &self.if_not_present_writes,
                &record_key,
                record_generation_id.as_ref(),
            )
            .await;

            match result {
                HandleIfNotPresentResult::Return(result) => {
                    // Concurrent put was faster, return it's result
                    return result;
                }
                HandleIfNotPresentResult::NeedPut(resolve) => {
                    // Now we are handling this record exclusively,
                    // check if already present in the database
                    let contains = self
                        .raw_db
                        .contains_existing_collection_record(
                            ContainsExistingCollectionRecordOptions {
                                record_key: record_key.as_ref(),
                            },
                        )
                        .await?;

                    match contains {
                        Some(record_key) => {
                            let record_key = record_key.as_ref();
                            let generation_id = record_key.get_generation_id();

                            resolve(HandleIfNotPresentResolve::AlreadyExists(generation_id));

                            return Ok(CollectionPutOk {
                                generation_id: generation_id.to_owned(),
                                was_put: true,
                            });
                        }
                        None => {
                            // We'll notify other `if_not_present` waiters about success of current put
                            resolve_put = Some(resolve);
                        }
                    }
                }
            }
        }

        let result = self
            .raw_db
            .put_collection_record(PutCollectionRecordOptions {
                record_key: record_key.as_ref(),
                value: update.value.as_ref().map(|x| x.as_ref()),
            })
            .await;

        let (result, if_not_present_result) = match result {
            Ok(_) => (
                Ok(CollectionPutOk {
                    generation_id: record_generation_id.to_owned(),
                    was_put: true,
                }),
                HandleIfNotPresentResolve::WasPut,
            ),
            Err(err) => (
                Err(CollectionMethodError::RawDb(err)),
                HandleIfNotPresentResolve::Err,
            ),
        };

        match resolve_put {
            Some(resolve) => {
                resolve(if_not_present_result);
            }
            None => {}
        }

        result
    }
}

enum HandleIfNotPresentResolve<'a> {
    WasPut,
    AlreadyExists(GenerationIdRef<'a>),
    Err,
}

enum HandleIfNotPresentResult<'a> {
    Return(CollectionPutResult),
    NeedPut(Box<dyn FnOnce(HandleIfNotPresentResolve<'_>) -> () + 'a>),
}

async fn handle_if_not_present<'a>(
    rw_hash: &'a std::sync::RwLock<HashMap<OwnedRecordKey, CollectionGenerationKeyStatus>>,
    key: &'a OwnedRecordKey,
    generation_id: GenerationIdRef<'a>,
) -> HandleIfNotPresentResult<'a> {
    'outer: loop {
        let mut keys = rw_hash.write().unwrap();

        let value = keys.get(&key);

        match value {
            Some(value) => {
                match value {
                    CollectionGenerationKeyStatus::InProgress(receiver) => {
                        let mut receiver = receiver.clone();

                        // Free lock, we'll wait for result
                        drop(keys);

                        let mut progress = receiver.borrow_and_update().clone();

                        // will be finished at some point
                        loop {
                            match progress {
                                CollectionGenerationKeyProgress::Pending => {
                                    let result = receiver.changed().await;

                                    match result {
                                        Ok(_) => {}
                                        Err(_err) => {
                                            return HandleIfNotPresentResult::Return(Err(
                                                CollectionMethodError::Channels,
                                            ));
                                        }
                                    }

                                    progress = receiver.borrow().clone();
                                }
                                CollectionGenerationKeyProgress::AlreadyExists(generation_id) => {
                                    return HandleIfNotPresentResult::Return(Ok(CollectionPutOk {
                                        generation_id,
                                        was_put: false,
                                    }));
                                }
                                CollectionGenerationKeyProgress::WasPut(generation_id) => {
                                    return HandleIfNotPresentResult::Return(Ok(CollectionPutOk {
                                        generation_id,
                                        was_put: true,
                                    }));
                                }
                                CollectionGenerationKeyProgress::Err => {
                                    // acquire lock again, key should be removed,
                                    // and we can try our attempt to put it
                                    continue 'outer;
                                }
                            }
                        }
                    }
                }
            }
            None => {
                let (sender, receiver) =
                    tokio::sync::watch::channel(CollectionGenerationKeyProgress::Pending);

                keys.insert(
                    key.clone(),
                    CollectionGenerationKeyStatus::InProgress(receiver),
                );

                return HandleIfNotPresentResult::NeedPut(Box::new(move |resolution| {
                    let mut keys = rw_hash.write().unwrap();
                    keys.remove(key);
                    drop(keys);

                    let value_to_send = match resolution {
                        HandleIfNotPresentResolve::WasPut => {
                            CollectionGenerationKeyProgress::WasPut(generation_id.to_owned())
                        }
                        HandleIfNotPresentResolve::AlreadyExists(generation_id) => {
                            CollectionGenerationKeyProgress::AlreadyExists(generation_id.to_owned())
                        }
                        HandleIfNotPresentResolve::Err => CollectionGenerationKeyProgress::Err,
                    };

                    sender.send(value_to_send).unwrap();
                }));
            }
        }
    }
}
