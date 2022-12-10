use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::put::inner::{
    CollectionPutInnerContinue, CollectionPutInnerOptions, CollectionPutInnerResult,
    HandleIfNotPresentResolve,
};
use crate::collection::util::record_flags::RecordFlags;
use crate::collection::util::record_key::OwnedRecordKey;
use crate::collection::Collection;
use crate::common::util::is_byte_array_equal_both_opt;
use crate::common::{GenerationId, GenerationIdRef, KeyValueUpdate, PhantomId};
use crate::generation::{CollectionGenerationKeyProgress, CollectionGenerationKeyStatus};
use crate::raw_db::contains_existing_collection_record::ContainsExistingCollectionRecordOptions;
use crate::raw_db::put_collection_record::PutCollectionRecordOptions;
use std::collections::HashMap;

mod inner;

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
        let update = &options.update;
        let generation_id = &options.generation_id;
        let phantom_id = &options.phantom_id;
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

        let inner_result = self
            .put_inner(CollectionPutInnerOptions {
                options: &options,
                record_generation_id: record_generation_id.as_ref(),
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
                record_key: record_key.as_ref().as_ref(),
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

        result
    }
}
