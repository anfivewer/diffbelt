use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::put::{CollectionPutOk, CollectionPutOptions, CollectionPutResult};
use crate::collection::util::record_key::OwnedRecordKey;
use crate::collection::Collection;
use crate::common::{GenerationId, GenerationIdRef, PhantomId, PhantomIdRef};
use crate::generation::{CollectionGenerationKeyProgress, CollectionGenerationKeyStatus};
use crate::raw_db::contains_existing_collection_record::ContainsExistingCollectionRecordOptions;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CollectionPutInnerOptions<'a> {
    pub options: &'a CollectionPutOptions,
    pub record_generation_id: GenerationIdRef<'a>,
}

pub struct CollectionPutInnerContinue<'a> {
    pub record_key: Arc<OwnedRecordKey>,
    pub resolve: Option<ResolvePutFn<'a>>,
}

pub enum CollectionPutInnerResult<'a> {
    Done(CollectionPutResult),
    Continue(CollectionPutInnerContinue<'a>),
}

impl Collection {
    pub async fn put_inner<'a>(
        &'a self,
        options: CollectionPutInnerOptions<'a>,
    ) -> Result<CollectionPutInnerResult<'a>, CollectionMethodError> {
        let method_options = options.options;
        let record_generation_id = options.record_generation_id;

        let update = &method_options.update;
        let key = &update.key;
        let phantom_id_or_empty = PhantomId::or_empty_as_ref(&update.phantom_id);

        let record_key =
            OwnedRecordKey::new(key.as_ref(), record_generation_id, phantom_id_or_empty)
                .or(Err(CollectionMethodError::InvalidKey))?;
        let record_key = Arc::new(record_key);

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
                                record_key: record_key.as_ref().as_ref(),
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
                                was_put: true,
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
                record_key: record_key.clone(),
                resolve: resolve_put,
            },
        ))
    }
}

pub enum HandleIfNotPresentResolve<'a> {
    WasPut,
    AlreadyExists(GenerationIdRef<'a>),
    Err,
}

type ResolvePutFn<'a> = Box<dyn FnOnce(HandleIfNotPresentResolve<'_>) -> () + 'a>;

enum HandleIfNotPresentResult<'a> {
    Return(CollectionPutResult),
    NeedPut(ResolvePutFn<'a>),
}

async fn handle_if_not_present<'a>(
    rw_hash: &'a std::sync::RwLock<HashMap<OwnedRecordKey, CollectionGenerationKeyStatus>>,
    key: Arc<OwnedRecordKey>,
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
                    key.as_ref().clone(),
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

                    sender.send(value_to_send).unwrap();
                }));
            }
        }
    }
}
