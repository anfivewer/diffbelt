use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::put::inner::{
    validate_put, CollectionPutInnerContinue, CollectionPutInnerOptions, CollectionPutInnerResult,
    HandleIfNotPresentResolve, ValidatePutOptions,
};
use crate::collection::Collection;

use crate::common::{KeyValueUpdate, OwnedGenerationId, OwnedPhantomId};
use crate::messages::generations::{
    DatabaseCollectionGenerationsTask, LockNextGenerationIdTask, LockNextGenerationIdTaskResponse,
};
use crate::raw_db::put_collection_record::PutCollectionRecordOptions;
use crate::util::async_sync_call::async_sync_call;

pub struct CollectionPutOptions {
    pub update: KeyValueUpdate,
    pub generation_id: Option<OwnedGenerationId>,
    pub phantom_id: Option<OwnedPhantomId>,
}

#[derive(Debug)]
pub struct CollectionPutOk {
    pub generation_id: OwnedGenerationId,
    // if `update.if_not_present == true`, it can be false when nothing was changed
    pub was_put: bool,
}

pub type CollectionPutResult = Result<CollectionPutOk, CollectionMethodError>;

impl Collection {
    pub async fn put(&self, options: CollectionPutOptions) -> CollectionPutResult {
        let CollectionPutOptions {
            update,
            generation_id,
            phantom_id,
        } = options;

        let phantom_id = phantom_id.as_ref().map(|id| id.as_ref());

        let LockNextGenerationIdTaskResponse {
            next_generation_id,
            lock: mut next_generation_id_lock,
        } = async_sync_call(|sender| {
            self.database_inner.add_generations_task(
                DatabaseCollectionGenerationsTask::LockNextGenerationId(LockNextGenerationIdTask {
                    collection_id: self.generations_id,
                    sender,
                    next_generation_id: generation_id.clone(),
                    is_phantom: phantom_id.is_some(),
                }),
            )
        })
        .await??;

        //// Validate
        let error = validate_put(ValidatePutOptions {
            is_manual_collection: self.is_manual,
            generation_id: generation_id.as_ref().map(|id| id.as_ref()),
            phantom_id,
            next_generation_id: Some(next_generation_id.as_ref()),
        });

        match error {
            Some(error) => {
                return Err(error);
            }
            None => {}
        }

        //// Insert
        let record_generation_id = generation_id.unwrap_or(next_generation_id);
        let record_generation_id = record_generation_id.as_ref();

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let inner_result = self
            .put_inner(CollectionPutInnerOptions {
                update: &update,
                record_generation_id,
                phantom_id,
            })
            .await?;

        let inner_result = match inner_result {
            CollectionPutInnerResult::Done(result) => {
                return result;
            }
            CollectionPutInnerResult::Continue(value) => value,
        };

        let CollectionPutInnerContinue {
            record_key,
            resolve,
        } = inner_result;

        let result = self
            .raw_db
            .put_collection_record(PutCollectionRecordOptions {
                record_key: record_key.as_ref(),
                value: update.value.as_ref().map(|x| x.as_ref()),
            })
            .await;

        next_generation_id_lock.set_need_schedule_next_generation();

        drop(next_generation_id_lock);

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

        match resolve {
            Some(resolve) => {
                resolve(if_not_present_result);
            }
            None => {}
        }

        drop(deletion_lock);

        result
    }
}
