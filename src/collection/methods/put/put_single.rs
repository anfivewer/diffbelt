use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::put::inner::{
    validate_put, CollectionPutInnerContinue, CollectionPutInnerOptions, CollectionPutInnerResult,
    HandleIfNotPresentResolve, ValidatePutOptions,
};
use crate::collection::Collection;

use crate::common::{
    GenerationId, GenerationIdRef, KeyValueUpdate, NeverEq, PhantomId, PhantomIdRef,
};
use crate::raw_db::put_collection_record::PutCollectionRecordOptions;

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

pub type CollectionPutResult = Result<CollectionPutOk, CollectionMethodError>;

impl Collection {
    pub async fn put(&self, options: CollectionPutOptions) -> CollectionPutResult {
        let update = &options.update;
        let generation_id: Option<GenerationIdRef> =
            options.generation_id.as_ref().map(|gen| gen.as_ref());
        let phantom_id: Option<PhantomIdRef> = options.phantom_id.as_ref().map(|id| id.as_ref());

        let next_generation_id = self.next_generation_id.read().unwrap();
        let next_generation_id: Option<GenerationIdRef> =
            next_generation_id.as_ref().map(|gen| gen.as_ref());

        //// Validate
        let error = validate_put(ValidatePutOptions {
            is_manual_collection: self.is_manual,
            generation_id,
            phantom_id: phantom_id.clone(),
            next_generation_id,
        });

        match error {
            Some(error) => {
                return Err(error);
            }
            None => {}
        }

        //// Insert
        let record_generation_id = generation_id
            .or(next_generation_id)
            .expect("Collection::put, no either generation_id or next_generation");

        let inner_result = self
            .put_inner(CollectionPutInnerOptions {
                update,
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

        self.on_put();

        result
    }
}
