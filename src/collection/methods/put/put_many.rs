use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::put::inner::{
    validate_put, CollectionPutInnerOptions, CollectionPutInnerResult,
    HandleIfNotPresentResolve, ResolvePutFn, ValidatePutOptions,
};

use crate::collection::Collection;

use crate::common::{GenerationId, GenerationIdRef, KeyValueUpdate, PhantomId, PhantomIdRef};

use crate::raw_db::put_many_collection_records::{
    PutManyCollectionRecordsItem, PutManyCollectionRecordsOptions,
};

pub struct CollectionPutManyOptions {
    pub items: Vec<KeyValueUpdate>,
    pub generation_id: Option<GenerationId>,
    pub phantom_id: Option<PhantomId>,
}

#[derive(Debug)]
pub struct CollectionPutManyOk {
    pub generation_id: GenerationId,
}

pub type CollectionPutManyResult = Result<CollectionPutManyOk, CollectionMethodError>;

impl Collection {
    pub async fn put_many(&self, options: CollectionPutManyOptions) -> CollectionPutManyResult {
        let generation_id: Option<GenerationIdRef> =
            options.generation_id.as_ref().map(|gen| gen.as_ref());
        let phantom_id: Option<PhantomIdRef> = options.phantom_id.as_ref().map(|id| id.as_ref());

        let next_generation = self.next_generation.read().await;
        let next_generation_id: Option<GenerationIdRef> =
            next_generation.as_ref().map(|gen| gen.id.as_ref());

        //// Validate
        let error = validate_put(ValidatePutOptions {
            is_manual_collection: self.is_manual,
            generation_id,
            phantom_id,
            next_generation_id,
        });

        match error {
            Some(error) => {
                return Err(error);
            }
            None => {}
        }

        let record_generation_id = generation_id
            .or(next_generation_id)
            .expect("Collection::put, no either generation_id or next_generation");

        // //// Insert
        let items_inner = options.items.into_iter().map(|update| async move {
            let result = self
                .put_inner(CollectionPutInnerOptions {
                    update: &update,
                    record_generation_id,
                    phantom_id: phantom_id.clone(),
                })
                .await;

            return (result, update);
        });

        let items_inner: Vec<(
            Result<CollectionPutInnerResult, CollectionMethodError>,
            KeyValueUpdate,
        )> = futures::future::join_all(items_inner).await;

        type AccumulatorVec<'a> = (Vec<PutManyCollectionRecordsItem>, Vec<ResolvePutFn<'a>>);

        type Accumulator<'a> =
            Result<AccumulatorVec<'a>, (CollectionMethodError, AccumulatorVec<'a>)>;

        let initial = (
            Vec::with_capacity(items_inner.len()),
            Vec::with_capacity(items_inner.len()),
        );
        let items_inner: Accumulator<'_> =
            items_inner
                .into_iter()
                .fold(Ok(initial), |acc, (item, update)| match item {
                    Ok(inner_result) => match inner_result {
                        CollectionPutInnerResult::Continue(cont) => match acc {
                            Ok((mut items_vec, mut resolve_vec)) => {
                                items_vec.push(PutManyCollectionRecordsItem {
                                    record_key: cont.record_key,
                                    value: update.value,
                                });
                                push_if(&mut resolve_vec, cont.resolve);
                                Ok((items_vec, resolve_vec))
                            }
                            Err((first_error, (items_vec, mut resolve_vec))) => {
                                push_if(&mut resolve_vec, cont.resolve);
                                Err((first_error, (items_vec, resolve_vec)))
                            }
                        },
                        CollectionPutInnerResult::Done(_done) => acc,
                    },
                    Err(err) => match acc {
                        Ok(acc_vec) => Err((err, acc_vec)),
                        err => err,
                    },
                });

        // Contains keys which needs to be put
        let (items, resolves) = match items_inner {
            Ok(inner_results) => inner_results,
            // In case of error locking some of records, cancel all locks
            Err((first_error, (_, resolve_vec))) => {
                for resolve in resolve_vec {
                    resolve(HandleIfNotPresentResolve::Err);
                }

                return Err(first_error);
            }
        };

        let result = self
            .raw_db
            .put_many_collection_records(PutManyCollectionRecordsOptions { items })
            .await;

        let (result, if_not_present_result) = match result {
            Ok(_) => (
                Ok(CollectionPutManyOk {
                    generation_id: record_generation_id.to_owned(),
                }),
                HandleIfNotPresentResolve::WasPut,
            ),
            Err(err) => (
                Err(CollectionMethodError::RawDb(err)),
                HandleIfNotPresentResolve::Err,
            ),
        };

        for resolve in resolves {
            resolve(if_not_present_result);
        }

        result
    }
}

#[inline]
fn push_if<T>(vec: &mut Vec<T>, opt: Option<T>) {
    match opt {
        Some(value) => {
            vec.push(value);
        }
        None => {}
    }
}
