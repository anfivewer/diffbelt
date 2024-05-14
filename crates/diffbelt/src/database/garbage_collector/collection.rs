use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use tokio::sync::{oneshot, watch, RwLock};
use tokio::task::{spawn_blocking, spawn_local};
use tokio::time::sleep;

use diffbelt_util::idling_status::IdlingStatus;

use crate::collection::util::collection_raw_db::CollectionRawDb;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::common::OwnedGenerationId;
use crate::database::config::DatabaseConfig;
use crate::raw_db::garbage_collector::{CleanupGenerationsLessThanOptions, CleanupResult};

pub struct GarbageCollectorCollection {
    pub id: usize,
    raw_db: CollectionRawDb,
    is_deleted: Arc<RwLock<bool>>,
    idling: IdlingStatus,
}

impl GarbageCollectorCollection {
    pub fn new(
        id: usize,
        raw_db: CollectionRawDb,
        is_deleted: Arc<RwLock<bool>>,
        idling: IdlingStatus,
    ) -> Self {
        Self {
            id,
            raw_db,
            is_deleted,
            idling,
        }
    }

    pub fn cleanup_generations_less_than(
        self: Rc<Self>,
        config: &DatabaseConfig,
        mut minimum_generation_id: watch::Receiver<OwnedGenerationId>,
        mut stop_receiver: oneshot::Receiver<()>,
    ) {
        let records_limit = config.gc_records_limit;
        let lookups_limit = config.gc_lookups_limit;
        let gc_sleep_duration = config.gc_sleep_duration;

        let raw_db = self.raw_db.clone();

        spawn_local(async move {
            let mut local_generation_less_than = OwnedGenerationId::empty();
            let mut continue_from_record_key = None;

            let check_generation =
                |minimum_generation_id: &mut watch::Receiver<OwnedGenerationId>,
                 local_generation_less_than: &mut OwnedGenerationId,
                 continue_from_record_key: &mut Option<OwnedRecordKey>| {
                    let current = minimum_generation_id.borrow_and_update();
                    let current = current.deref();

                    if current > local_generation_less_than {
                        *local_generation_less_than = current.clone();
                        *continue_from_record_key = None;
                    }
                };

            let mut busy_task = None;

            loop {
                if busy_task.is_none() {
                    busy_task = Some(self.idling.start_work());
                }

                check_generation(
                    &mut minimum_generation_id,
                    &mut local_generation_less_than,
                    &mut continue_from_record_key,
                );

                let result = {
                    let raw_db = raw_db.clone();
                    let local_generation_less_than = local_generation_less_than.clone();

                    let is_deleted = self.is_deleted.read().await;
                    if *is_deleted {
                        return;
                    }

                    spawn_blocking(move || {
                        let result = raw_db
                            .cleanup_generations_less_than_sync(CleanupGenerationsLessThanOptions {
                                generation_less_than: local_generation_less_than.as_ref(),
                                continue_from_record_key,
                                records_limit,
                                lookups_limit,
                            })
                            .expect("garbage_collector:raw_db:cleanup_generations_less_than_sync");

                        result
                    })
                    .await
                    .expect("garbage_collector:join")
                };

                // Slowdown cleanups to not use too much cpu, evently it will be clean
                let sleep_future = sleep(gc_sleep_duration);
                tokio::select! {
                    () = sleep_future => {},
                    _ = &mut stop_receiver => {
                        return;
                    }
                };

                match result {
                    CleanupResult::NeedToContinue(continuation) => {
                        continue_from_record_key = continuation;
                    }
                    CleanupResult::Finished => {
                        continue_from_record_key = None;
                        busy_task = None;

                        tokio::select! {
                            result = minimum_generation_id.changed() => {
                                match result {
                                    Ok(_) => {},
                                    Err(_) => {
                                        return;
                                    },
                                }
                            },
                            _ = &mut stop_receiver => {
                                return;
                            }
                        };
                    }
                }
            }
        });
    }
}
