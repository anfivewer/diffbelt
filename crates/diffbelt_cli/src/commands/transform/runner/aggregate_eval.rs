use std::cell::RefCell;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use diffbelt_cli_config::errors::RunTransformError;
use diffbelt_cli_config::transforms::aggregate::Aggregate;
use diffbelt_cli_config::wasm::aggregate::AggregateFunctions;
use diffbelt_cli_config::wasm::memory::vector::WasmVecHolder;
use diffbelt_cli_config::wasm::WasmModuleInstance;
use diffbelt_protos::protos::impls::AggregateTargetInfoProto;
use diffbelt_protos::{OwnedSerialized, SerializedRawParts};
use diffbelt_transforms::base::action::function_eval::{
    AggregateApplyEvalAction, AggregateInitialAccumulatorEvalAction, AggregateMapEvalAction,
    AggregateMergeEvalAction, AggregateReduceEvalAction, AggregateTargetInfoEvalAction,
    FunctionEvalAction,
};
use diffbelt_transforms::base::common::accumulator::AccumulatorId;
use diffbelt_transforms::base::common::target_info::TargetInfoId;
use diffbelt_transforms::base::input::function_eval::{
    AggregateApplyEvalInput, AggregateInitialAccumulatorEvalInput, AggregateMapEvalInput,
    AggregateMergeEvalInput, AggregateReduceEvalInput, AggregateTargetInfoEvalInput,
    FunctionEvalInput, FunctionEvalInputBody,
};
use diffbelt_transforms::Transform;
use diffbelt_util_no_std::cast::{u32_to_u64, u64_to_usize, usize_to_u64};
use diffbelt_util_no_std::temporary_collection::vec::{TempVecType, TemporaryVec};
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;
use generational_arena::{Arena, Index};

use crate::commands::errors::{CommandError, TransformEvalError, WasmAggregateMapCallError};
use crate::commands::transform::runner::function_eval_handler::FunctionEvalHandler;
use crate::commands::transform::runner::transform_config::AggregateTransformInfo;
use crate::commands::transform::runner::InputEmitter;

pub struct AggregateEvalHandler {
    inner: Inner,
    inner_mut: RefCell<InnerMut>,
    instance: Pin<Box<WasmModuleInstance>>,
}

struct Inner {
    aggregate_functions: AggregateFunctions<'static>,
}

struct InnerMut {
    target_info_arena: Arena<OwnedSerialized<AggregateTargetInfoProto>>,
    accumulators_arena: Arena<AccumulatorInfo>,
    free_accumulator_indexes: Vec<usize>,
    temp_merge_vec: TemporaryVec<WasmVecHolderTemp>,
}

struct AccumulatorInfo {
    target_info_index: Index,
    wasm_vec_holder: WasmVecHolder<'static>,
}

struct WasmVecHolderTemp;

impl TempVecType for WasmVecHolderTemp {
    type Item<'a> = &'a WasmVecHolder<'a>;
}

impl AggregateEvalHandler {
    pub async fn new(
        instance: WasmModuleInstance,
        aggregate: &AggregateTransformInfo,
    ) -> Result<Self, RunTransformError> {
        let instance = Box::pin(instance);
        let instance_static = unsafe {
            mem::transmute::<&WasmModuleInstance, &'static WasmModuleInstance>(instance.deref())
        };

        let aggregate_functions = AggregateFunctions::new(
            instance_static,
            aggregate.map.as_str(),
            aggregate.initial_accumulator.as_str(),
            aggregate.reduce.as_str(),
            aggregate.merge_accumulators.as_ref().map(|x| x.as_str()),
            aggregate.apply.as_str(),
        )
        .await?;

        Ok(Self {
            inner: Inner {
                aggregate_functions,
            },
            inner_mut: RefCell::new(InnerMut {
                target_info_arena: Arena::with_capacity(32),
                accumulators_arena: Arena::with_capacity(32),
                free_accumulator_indexes: Vec::with_capacity(8),
                temp_merge_vec: TemporaryVec::new(),
            }),
            instance,
        })
    }
}

impl FunctionEvalHandler for AggregateEvalHandler {
    async fn handle_action(
        &self,
        action: FunctionEvalAction,
        input_emitter: InputEmitter,
        transform: Arc<Mutex<impl Transform>>,
    ) {
        let body = (|| async move {
            match action {
                FunctionEvalAction::AggregateMap(action) => {
                    let AggregateMapEvalAction { input } = action;

                    let output_buffer = {
                        let mut transform = transform.lock().expect("lock");
                        transform.take_map_input_buffer()
                    };
                    let mut output_holder = Some(output_buffer);

                    let result = self
                        .inner
                        .aggregate_functions
                        .call_map(
                            FlatbufferAnnotated::from(input.as_bytes()),
                            &mut output_holder,
                        )
                        .await;

                    let result = match result {
                        Ok(x) => x,
                        Err(err) => {
                            let SerializedRawParts { buffer, head, len } = input.into_raw_parts();

                            return Err(TransformEvalError::WasmAggregateMapCall(
                                WasmAggregateMapCallError {
                                    input_buffer: buffer,
                                    input_head: head,
                                    input_len: len,
                                    error: err,
                                },
                            ));
                        }
                    };

                    {
                        let mut transform = transform.lock().expect("lock");
                        transform.return_map_action_buffer(input.into_buffer_vec());
                    }

                    Ok(FunctionEvalInputBody::AggregateMap(AggregateMapEvalInput {
                        input: result,
                    }))
                }
                FunctionEvalAction::AggregateTargetInfo(action) => {
                    let AggregateTargetInfoEvalAction { target_info } = action;

                    let target_info_data_bytes = usize_to_u64(target_info.as_bytes().len());

                    let target_info_id = {
                        let mut inner_mut = self.inner_mut.borrow_mut();
                        let target_info_arena = &mut inner_mut.target_info_arena;
                        let index = target_info_arena.insert(target_info);
                        let (index, _generation) = index.into_raw_parts();
                        TargetInfoId(usize_to_u64(index))
                    };

                    Ok(FunctionEvalInputBody::AggregateTargetInfo(
                        AggregateTargetInfoEvalInput {
                            target_info_id,
                            target_info_data_bytes,
                        },
                    ))
                }
                FunctionEvalAction::AggregateInitialAccumulator(action) => {
                    let AggregateInitialAccumulatorEvalAction { target_info } = action;

                    let target_info_index = u64_to_usize(target_info.0);
                    let (accumulator_id, accumulator_data_bytes) = {
                        let mut inner_mut = self.inner_mut.borrow_mut();
                        let InnerMut {
                            target_info_arena,
                            accumulators_arena,
                            free_accumulator_indexes,
                            ..
                        } = inner_mut.deref_mut();

                        let (target_info, target_info_index) = target_info_arena
                            .get_unknown_gen(target_info_index)
                            .ok_or_else(|| {
                                TransformEvalError::Unspecified("No target info record".to_string())
                            })?;

                        let free_accumulator_index = free_accumulator_indexes.pop();

                        let (index, accumulator_info) = match free_accumulator_index {
                            Some(index) => {
                                let (accumulator, _) = accumulators_arena
                                    .get_unknown_gen_mut(index)
                                    .ok_or_else(|| {
                                        TransformEvalError::Unspecified(
                                            "No item at free accumulator index".to_string(),
                                        )
                                    })?;

                                accumulator.target_info_index = target_info_index;

                                (index, accumulator as &AccumulatorInfo)
                            }
                            None => {
                                let wasm_vec_holder =
                                    self.instance_static().alloc_vec_holder().await?;

                                let index = accumulators_arena.insert(AccumulatorInfo {
                                    target_info_index,
                                    wasm_vec_holder,
                                });

                                let accumulator =
                                    accumulators_arena.get(index).expect("just inserted");

                                let (index, _generation) = index.into_raw_parts();

                                (index, accumulator)
                            }
                        };

                        let () = self
                            .inner
                            .aggregate_functions
                            .call_initial_accumulator(
                                FlatbufferAnnotated::from(target_info.as_bytes()),
                                &accumulator_info.wasm_vec_holder,
                            )
                            .await?;

                        let accumulator_data_bytes =
                            u32_to_u64(accumulator_info.wasm_vec_holder.read_slice()?.0.len);

                        (AccumulatorId(usize_to_u64(index)), accumulator_data_bytes)
                    };

                    Ok(FunctionEvalInputBody::AggregateInitialAccumulator(
                        AggregateInitialAccumulatorEvalInput {
                            accumulator_id,
                            accumulator_data_bytes,
                        },
                    ))
                }
                FunctionEvalAction::AggregateReduce(action) => {
                    let AggregateReduceEvalAction {
                        accumulator: accumulator_id,
                        input,
                    } = action;

                    let accumulator_data_bytes = {
                        let inner_mut = self.inner_mut.borrow();
                        let (accumulator, _) = inner_mut
                            .accumulators_arena
                            .get_unknown_gen(u64_to_usize(accumulator_id.0))
                            .ok_or_else(|| {
                                TransformEvalError::Unspecified(
                                    "No accumulator at index".to_string(),
                                )
                            })?;

                        let () = self
                            .inner
                            .aggregate_functions
                            .call_reduce(
                                FlatbufferAnnotated::from(input.as_bytes()),
                                &accumulator.wasm_vec_holder,
                            )
                            .await?;

                        let accumulator_data_bytes =
                            u32_to_u64(accumulator.wasm_vec_holder.read_slice()?.0.len);

                        accumulator_data_bytes
                    };

                    {
                        let mut transform = transform.lock().expect("lock");
                        transform.return_reduce_action_buffer(input.into_buffer_vec());
                    }

                    Ok(FunctionEvalInputBody::AggregateReduce(
                        AggregateReduceEvalInput {
                            accumulator_id,
                            accumulator_data_bytes,
                        },
                    ))
                }
                FunctionEvalAction::AggregateMerge(action) => {
                    let AggregateMergeEvalAction { accumulator_ids } = action;

                    let mut accumulator_ids_iter = accumulator_ids.iter();

                    let first_accumulator_id = accumulator_ids_iter
                        .next()
                        .ok_or_else(|| {
                            TransformEvalError::Unspecified("No accumulators for merge".to_string())
                        })?
                        .clone();

                    let accumulator_data_bytes = {
                        let mut inner_mut = self.inner_mut.borrow_mut();
                        let InnerMut {
                            accumulators_arena,
                            free_accumulator_indexes,
                            temp_merge_vec,
                            ..
                        } = inner_mut.deref_mut();

                        let (first_accumulator, _) = accumulators_arena
                            .get_unknown_gen(u64_to_usize(first_accumulator_id.0))
                            .ok_or_else(|| {
                                TransformEvalError::Unspecified(
                                    "No accumulator at index".to_string(),
                                )
                            })?;

                        let mut accumulators_temp = temp_merge_vec.temp();
                        let accumulators = accumulators_temp.as_mut();

                        accumulators.reserve(accumulator_ids_iter.len());

                        for id in accumulator_ids_iter {
                            let (accumulator_holder, _) = accumulators_arena
                                .get_unknown_gen(u64_to_usize(id.0))
                                .ok_or_else(|| {
                                    TransformEvalError::Unspecified(
                                        "No accumulator at index".to_string(),
                                    )
                                })?;

                            accumulators.push(&accumulator_holder.wasm_vec_holder);

                            free_accumulator_indexes.push(u64_to_usize(id.0));
                        }

                        let () = self
                            .inner
                            .aggregate_functions
                            .call_merge_accumulators(
                                accumulators.iter(),
                                &first_accumulator.wasm_vec_holder,
                            )
                            .await?;

                        let accumulator_data_bytes =
                            u32_to_u64(first_accumulator.wasm_vec_holder.read_slice()?.0.len);

                        accumulator_data_bytes
                    };

                    {
                        let mut transform = transform.lock().expect("lock");
                        transform.return_merge_accumulator_ids_vec(accumulator_ids);
                    }

                    Ok(FunctionEvalInputBody::AggregateMerge(
                        AggregateMergeEvalInput {
                            accumulator_id: first_accumulator_id,
                            accumulator_data_bytes,
                        },
                    ))
                }
                FunctionEvalAction::AggregateApply(action) => {
                    let AggregateApplyEvalAction {
                        accumulator: accumulator_id,
                    } = action;

                    let input = {
                        let mut inner_mut = self.inner_mut.borrow_mut();
                        let InnerMut {
                            target_info_arena,
                            accumulators_arena,
                            free_accumulator_indexes,
                            ..
                        } = inner_mut.deref_mut();

                        let (accumulator, _) = accumulators_arena
                            .get_unknown_gen(u64_to_usize(accumulator_id.0))
                            .ok_or_else(|| {
                                TransformEvalError::Unspecified(
                                    "No accumulator at index".to_string(),
                                )
                            })?;

                        let buffer = {
                            let mut transform = transform.lock().expect("lock");
                            transform.take_apply_input_buffer()
                        };
                        let mut buffer_holder = Some(buffer);

                        let apply_output = self
                            .inner
                            .aggregate_functions
                            .call_apply(&accumulator.wasm_vec_holder, &mut buffer_holder)
                            .await?;

                        free_accumulator_indexes.push(u64_to_usize(accumulator_id.0));

                        let target_info = target_info_arena
                            .remove(accumulator.target_info_index)
                            .ok_or_else(|| {
                            TransformEvalError::Unspecified("No target info at index".to_string())
                        })?;

                        {
                            let mut transform = transform.lock().expect("lock");
                            transform
                                .return_target_info_action_buffer(target_info.into_buffer_vec());
                        }

                        apply_output
                    };

                    Ok(FunctionEvalInputBody::AggregateApply(
                        AggregateApplyEvalInput { input },
                    ))
                }
                _ => Err(TransformEvalError::Unspecified(format!(
                    "Unsupported aggregate action: {action:?}"
                ))),
            }
        })()
        .await;

        let () = input_emitter
            .emit_input(body.map(|body| FunctionEvalInput { body }))
            .await;
    }
}

impl AggregateEvalHandler {
    fn instance_static(&self) -> &'static WasmModuleInstance {
        let instance_static = unsafe {
            mem::transmute::<&WasmModuleInstance, &'static WasmModuleInstance>(
                self.instance.deref(),
            )
        };

        instance_static
    }
}
