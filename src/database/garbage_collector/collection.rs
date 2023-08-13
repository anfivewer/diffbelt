use crate::collection::util::collection_raw_db::CollectionRawDb;
use crate::common::OwnedGenerationId;
use crate::database::config::DatabaseConfig;
use crate::raw_db::garbage_collector::{CleanupGenerationsLessThanOptions, CleanupResult};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::{spawn_blocking, spawn_local, yield_now};

pub struct GarbageCollectorCollection {
    pub id: usize,
    raw_db: CollectionRawDb,
    is_deleted: Arc<RwLock<bool>>,
    generation_less_than: RefCell<Option<OwnedGenerationId>>,
    is_cleaning: Cell<bool>,
}

impl GarbageCollectorCollection {
    pub fn new(id: usize, raw_db: CollectionRawDb, is_deleted: Arc<RwLock<bool>>) -> Self {
        Self {
            id,
            raw_db,
            is_deleted,
            generation_less_than: RefCell::new(None),
            is_cleaning: Cell::new(false),
        }
    }

    pub fn cleanup_generations_less_than(
        self: Rc<Self>,
        config: &DatabaseConfig,
        generation_less_than: OwnedGenerationId,
    ) {
        let is_generation_id_greater_than_current = 'block: {
            let current = self.generation_less_than.borrow();
            let Some(current) = current.as_ref() else{
                break 'block true;
            };

            &generation_less_than > current
        };

        if is_generation_id_greater_than_current {
            let mut current = self.generation_less_than.borrow_mut();
            current.replace(generation_less_than.clone());
        }

        if self.is_cleaning.get() {
            return;
        }

        self.is_cleaning.set(true);

        let records_limit = config.gc_records_limit;
        let lookups_limit = config.gc_lookups_limit;

        let raw_db = self.raw_db.clone();

        spawn_local(async move {
            let mut local_generation_less_than = generation_less_than;
            let mut continue_from_record_key = None;

            let check_generation = |local_generation_less_than: &mut OwnedGenerationId| {
                let current = self.generation_less_than.borrow();
                let Some(current) = current.as_ref() else {
                    return false;
                };

                if current > local_generation_less_than {
                    *local_generation_less_than = current.clone();
                    return true;
                }

                return false;
            };

            loop {
                let is_deleted = self.is_deleted.read().await;
                if *is_deleted {
                    return;
                }

                check_generation(&mut local_generation_less_than);

                let result = {
                    let raw_db = raw_db.clone();
                    let local_generation_less_than = local_generation_less_than.clone();
                    spawn_blocking(move || {
                        raw_db
                            .cleanup_generations_less_than_sync(CleanupGenerationsLessThanOptions {
                                generation_less_than: local_generation_less_than.as_ref(),
                                continue_from_record_key,
                                records_limit,
                                lookups_limit,
                            })
                            .expect("garbage_collector:raw_db:cleanup_generations_less_than_sync")
                    })
                    .await
                    .expect("garbage_collector:join")
                };

                match result {
                    CleanupResult::NeedToContinue(continuation) => {
                        continue_from_record_key = continuation;
                    }
                    CleanupResult::Finished => {
                        if !check_generation(&mut local_generation_less_than) {
                            self.is_cleaning.set(false);
                            return;
                        }

                        continue_from_record_key = None;
                    }
                }

                yield_now().await;
            }
        });
    }
}
