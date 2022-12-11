use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::put::{CollectionPutOk, CollectionPutResult};
use crate::collection::util::record_key::OwnedRecordKey;
use crate::collection::Collection;
use crate::common::util::is_byte_array_equal_both_opt;
use crate::common::{GenerationIdRef, KeyValueUpdate, NeverEq, PhantomIdRef};
use crate::generation::{CollectionGenerationKeyProgress, CollectionGenerationKeyStatus};
use crate::raw_db::contains_existing_collection_record::ContainsExistingCollectionRecordOptions;
use std::borrow::Borrow;
use std::collections::HashMap;

pub struct ValidatePutOptions<'a> {
    pub is_manual_collection: bool,
    pub generation_id: Option<GenerationIdRef<'a>>,
    pub phantom_id: Option<PhantomIdRef<'a>>,
    pub next_generation_id: Option<GenerationIdRef<'a>>,
}

pub fn validate_put(options: ValidatePutOptions<'_>) -> Option<CollectionMethodError> {
    let generation_id = options.generation_id;
    let next_generation_id = options.next_generation_id;
    let phantom_id = options.phantom_id;
    let is_phantom = phantom_id.is_some();

    if is_phantom && generation_id.is_none() {
        // Phantom writes can be only to the specified generation
        return Some(CollectionMethodError::PutPhantomWithoutGenerationId);
    }

    let is_generation_id_equal_to_next_one =
        is_byte_array_equal_both_opt(generation_id, next_generation_id);

    // Phantom puts are allowed to do everything (except to be without a specified generationId),
    // but we are already checked it above
    if !is_phantom {
        if generation_id.is_some() {
            if !is_generation_id_equal_to_next_one {
                return Some(CollectionMethodError::OutdatedGeneration);
            }
        } else if options.is_manual_collection {
            // we cannot put values is manual collection without specified generationId
            return Some(CollectionMethodError::CannotPutInManualCollection);
        } else if next_generation_id.is_none() {
            panic!("Collection::put, no next_generation in !manual collection");
        }
    }

    None
}

pub struct CollectionPutInnerOptions<'a, 'b> {
    pub update: &'b KeyValueUpdate,
    pub record_generation_id: GenerationIdRef<'a>,
    pub phantom_id: Option<PhantomIdRef<'a>>,
}

pub struct CollectionPutInnerContinue<'a> {
    pub record_key: OwnedRecordKey,
    pub resolve: Option<ResolvePutFn<'a>>,
}

pub enum CollectionPutInnerResult<'a> {
    Done(CollectionPutResult),
    Continue(CollectionPutInnerContinue<'a>),
}

impl Collection {
    pub async fn put_inner<'a, 'b>(
        &'a self,
        options: CollectionPutInnerOptions<'a, 'b>,
    ) -> Result<CollectionPutInnerResult<'a>, CollectionMethodError> {
        let update = options.update;
        let key = update.key.as_ref();
        let phantom_id = options.phantom_id;
        let record_generation_id = options.record_generation_id;

        let phantom_id_or_empty = PhantomIdRef::or_empty(&phantom_id);

        let record_key = OwnedRecordKey::new(key, record_generation_id, phantom_id_or_empty)
            .or(Err(CollectionMethodError::InvalidKey))?;

        let mut resolve_put = None;

        // When `if_not_present = true`, we need to not write same record in the same time,
        // to provide correct response from this method
        // (when there is two concurrent puts, we should return `was_put = true` only in single one)
        if update.if_not_present {
            // Check hashmap of current puts or take a put lock
            let result = handle_if_not_present(
                &self.if_not_present_writes,
                record_key.clone(),
                record_generation_id,
            )
            .await;

            match result {
                HandleIfNotPresentResult::Return(result) => {
                    // Concurrent put was faster, return it's result
                    return Ok(CollectionPutInnerResult::Done(result));
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

                            return Ok(CollectionPutInnerResult::Done(Ok(CollectionPutOk {
                                generation_id: generation_id.to_owned(),
                                was_put: false,
                            })));
                        }
                        None => {
                            // We'll notify other `if_not_present` waiters about success of current put
                            resolve_put = Some(resolve);
                        }
                    }
                }
            }
        }

        Ok(CollectionPutInnerResult::Continue(
            CollectionPutInnerContinue {
                record_key,
                resolve: resolve_put,
            },
        ))
    }

    pub fn on_put(&self) {
        match self.on_put_sender.as_ref() {
            Some(sender) => {
                sender.send(NeverEq).unwrap_or(());
            }
            None => {}
        }
    }
}

#[derive(Copy, Clone)]
pub enum HandleIfNotPresentResolve<'a> {
    WasPut,
    AlreadyExists(GenerationIdRef<'a>),
    Err,
}

pub type ResolvePutFn<'a> = Box<dyn FnOnce(HandleIfNotPresentResolve<'_>) -> () + 'a>;

enum HandleIfNotPresentResult<'a> {
    Return(CollectionPutResult),
    NeedPut(ResolvePutFn<'a>),
}

async fn handle_if_not_present<'a>(
    rw_hash: &'a std::sync::RwLock<HashMap<OwnedRecordKey, CollectionGenerationKeyStatus>>,
    key: OwnedRecordKey,
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
                    keys.remove(&key);
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

                    sender.send_replace(value_to_send);
                }));
            }
        }
    }
}
