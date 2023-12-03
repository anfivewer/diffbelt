use std::borrow::Cow;
use std::collections::VecDeque;
use std::mem;
use std::ops::DerefMut;
use std::rc::Rc;

use diffbelt_protos::protos::transform::aggregate::{
    AggregateReduceInput, AggregateReduceInputArgs, AggregateReduceItem, AggregateReduceItemArgs,
};
use diffbelt_protos::{Serializer, WIPOffset};
use diffbelt_types::collection::get_record::GetRequestJsonData;
use diffbelt_types::common::key_value::EncodedKeyJsonData;
use diffbelt_util_no_std::buffers_pool::BuffersPool;

use crate::aggregate::context::{HandlerContext, MapContext, ReducingContext, TargetRecordContext};
use crate::aggregate::state::{
    ProcessingState, TargetKeyChunk, TargetKeyCollectingChunk, TargetKeyData,
    TargetKeyReducingChunk,
};
use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::function_eval::{
    AggregateInitialAccumulatorEvalAction, AggregateReduceEvalAction, FunctionEvalAction,
};
use crate::base::action::ActionType;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use crate::base::error::TransformError;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::base::input::function_eval::{AggregateMapEvalInput, FunctionEvalInput};
use crate::input_handler;
use crate::transform::{ActionInputHandlerActionsVec, ActionInputHandlerResult, HandlerResult};

impl AggregateTransform {
    pub fn on_map_received(
        &mut self,
        ctx: MapContext,
        map: AggregateMapEvalInput,
    ) -> HandlerResult<Self, HandlerContext> {
        let supports_accumulator_merge = self.supports_accumulator_merge;

        let state = self.state.expect_processing_mut()?;

        let MapContext { bytes_to_free } = ctx;
        state.current_limits.pending_eval_map_bytes -= bytes_to_free;

        let AggregateMapEvalInput {
            input,
            action_input_buffer,
        } = map;

        self.free_map_eval_action_buffers.push(action_input_buffer);

        let map_output = input.data();
        let map_items = map_output.items().unwrap_or_default();

        let mut prev_key_target: Option<(&[u8], &mut TargetKeyData)> = None;

        let mut updated_keys_temp = state.updated_target_keys_temp_set.temp();
        let updated_keys = updated_keys_temp.as_mut();

        for item in map_items {
            let target_key = item
                .target_key()
                .ok_or_else(|| TransformError::Unspecified("target_key is None".to_string()))?
                .bytes();
            let mapped_value = item.mapped_value();

            let target: &mut TargetKeyData = 'get_target: {
                if let Some((prev_target_key, target)) = &mut prev_key_target {
                    if *prev_target_key == target_key {
                        break 'get_target target.deref_mut();
                    }
                }

                let target = state.target_keys.get_mut(target_key);
                let target = if let Some(target) = target {
                    target
                } else {
                    state.target_keys.push(
                        Rc::from(target_key),
                        TargetKeyData {
                            target_info_id: None,
                            chunks: VecDeque::with_capacity(1),
                            is_target_info_pending: false,
                        },
                    );
                    state
                        .target_keys
                        .get_mut(target_key)
                        .expect("should be inserted")
                };

                prev_key_target = Some((target_key, target));
                let Some((_, target)) = &mut prev_key_target else {
                    panic!()
                };

                target.deref_mut()
            };

            let mut should_add_to_updated_keys = !target.is_target_info_pending;

            //region Add mapped item to target key record
            let last_chunk = target.chunks.back_mut();
            let last_chunk = match last_chunk {
                Some(last_chunk) => {
                    if let Some(chunk) = last_chunk.as_collecting() {
                        if chunk.is_accumulator_pending {
                            should_add_to_updated_keys = false
                        }

                        if chunk.is_reducing {
                            // We are in !supports_accumulator_merge mode, wait for previous reduce
                            should_add_to_updated_keys = false;
                        }
                    }

                    last_chunk
                }
                None => {
                    let mut buffer = self.free_reduce_eval_action_buffers.take();
                    buffer.clear();

                    let chunk = TargetKeyChunk::Collecting(TargetKeyCollectingChunk {
                        accumulator_id: None,
                        reduce_input: Serializer::from_vec(buffer),
                        reduce_input_items: self.free_serializer_reduce_input_items_buffers.take(),
                        is_accumulator_pending: false,
                        is_reducing: false,
                    });
                    target.chunks.push_back(chunk);
                    target.chunks.back_mut().expect("just inserted")
                }
            };

            let last_chunk = match last_chunk {
                TargetKeyChunk::Collecting(x) => x,
                TargetKeyChunk::Reducing(_) | TargetKeyChunk::Reduced(_) => {
                    if !supports_accumulator_merge {
                        panic!("aggregate transform without support of accumulator merge cannot have Reducing/Reduced target key state");
                    }

                    let chunk = TargetKeyChunk::Collecting(TargetKeyCollectingChunk {
                        accumulator_id: None,
                        reduce_input: Serializer::from_vec(
                            self.free_reduce_eval_action_buffers.take(),
                        ),
                        reduce_input_items: self.free_serializer_reduce_input_items_buffers.take(),
                        is_accumulator_pending: false,
                        is_reducing: false,
                    });
                    target.chunks.push_back(chunk);
                    target.chunks.back_mut().expect("just inserted").as_collecting_mut().expect("aggregate transform without support of accumulator merge cannot have Reducing target key state")
                }
            };

            if should_add_to_updated_keys {
                updated_keys.insert(target_key);
            }

            let prev_buffer_len = last_chunk.reduce_input.buffer_len();

            let mapped_value =
                mapped_value.map(|x| last_chunk.reduce_input.create_vector(x.bytes()));

            let item = AggregateReduceItem::create(
                last_chunk.reduce_input.buffer_builder(),
                &AggregateReduceItemArgs { mapped_value },
            );

            last_chunk.reduce_input_items.push(item);

            let new_buffer_len = last_chunk.reduce_input.buffer_len();
            let new_data_size = new_buffer_len - prev_buffer_len;

            state.current_limits.target_data_bytes += new_data_size;
            //endregion
        }

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        for target_key in updated_keys.iter() {
            let target_key = *target_key;

            let (target_key_rc, _) = state
                .target_keys
                .get_key_value(target_key)
                .expect("should be present");
            let target_key_rc = target_key_rc.clone();

            let target = state
                .target_keys
                .get_mut(target_key)
                .expect("should be present");

            assert!(
                !target.is_target_info_pending,
                "Target info should not be pending"
            );

            let Some(target_info_id) = target.target_info_id else {
                target.is_target_info_pending = true;

                actions.push((
                    ActionType::DiffbeltCall(DiffbeltCallAction {
                        method: Method::Post,
                        path: Cow::Owned(format!(
                            "/collections/{}/get",
                            urlencoding::encode(&self.to_collection_name),
                        )),
                        query: Vec::with_capacity(0),
                        body: DiffbeltRequestBody::GetRecord(GetRequestJsonData {
                            key: EncodedKeyJsonData::from_bytes_slice(target_key),
                            generation_id: Some(state.to_generation_id.clone()),
                            phantom_id: None,
                        }),
                    }),
                    HandlerContext::TargetRecord(TargetRecordContext {
                        target_key: target_key_rc.clone(),
                    }),
                    input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                        let ctx = ctx.into_target_record().expect("should be TargetRecord");
                        let DiffbeltCallInput { body } = input.into_diffbelt_get_record()?;
                        this.on_target_record_received(ctx, body)
                    }),
                ));
                continue;
            };

            on_target_info_available(
                target_key_rc,
                &mut actions,
                target,
                target_info_id,
                supports_accumulator_merge,
                &mut state.reducing_chunk_id_counter,
                &mut self.free_reduce_eval_action_buffers,
                &mut self.free_serializer_reduce_input_items_buffers,
            );
        }

        Ok(ActionInputHandlerResult::AddActions(actions))
    }
}

pub fn on_target_info_available(
    target_key_rc: Rc<[u8]>,
    actions: &mut ActionInputHandlerActionsVec<AggregateTransform, HandlerContext>,
    target: &mut TargetKeyData,
    target_info_id: TargetInfoId,
    supports_accumulator_merge: bool,
    reducing_chunk_id_counter: &mut u64,
    free_reduce_eval_action_buffers: &mut BuffersPool<Vec<u8>>,
    free_serializer_reduce_input_items_buffers: &mut BuffersPool<
        Vec<WIPOffset<AggregateReduceItem<'static>>>,
    >,
) {
    let last_chunk = target
        .chunks
        .back_mut()
        .expect("chunks should not be empty");
    let collecting_chunk = last_chunk
        .as_collecting_mut()
        .expect("last chunk should be Collecting");

    assert_eq!(
        collecting_chunk.is_accumulator_pending,
        collecting_chunk.accumulator_id.is_some(),
        "accumulator should be pending or accumulator should be absent"
    );

    let Some(accumulator_id) = collecting_chunk.accumulator_id else {
        collecting_chunk.is_accumulator_pending = true;

        actions.push((
            ActionType::FunctionEval(FunctionEvalAction::AggregateInitialAccumulator(
                AggregateInitialAccumulatorEvalAction {
                    target_info: target_info_id,
                },
            )),
            HandlerContext::TargetRecord(TargetRecordContext {
                target_key: target_key_rc,
            }),
            input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
                let ctx = ctx.into_target_record().expect("should be TargetRecord");
                let FunctionEvalInput { body } = input.into_eval_aggregate_initial_accumulator()?;
                this.on_initial_accumulator_received(ctx, body)
            }),
        ));
        return;
    };

    reduce_target_chunk(
        target_key_rc,
        actions,
        last_chunk,
        target_info_id,
        accumulator_id,
        supports_accumulator_merge,
        reducing_chunk_id_counter,
        free_reduce_eval_action_buffers,
        free_serializer_reduce_input_items_buffers,
    );
}

pub fn reduce_target_chunk(
    target_key_rc: Rc<[u8]>,
    actions: &mut ActionInputHandlerActionsVec<AggregateTransform, HandlerContext>,
    last_chunk: &mut TargetKeyChunk,
    target_info_id: TargetInfoId,
    accumulator_id: AccumulatorId,
    supports_accumulator_merge: bool,
    reducing_chunk_id_counter: &mut u64,
    free_reduce_eval_action_buffers: &mut BuffersPool<Vec<u8>>,
    free_serializer_reduce_input_items_buffers: &mut BuffersPool<
        Vec<WIPOffset<AggregateReduceItem<'static>>>,
    >,
) {
    let (new_chunk_id, mut new_chunk) = if supports_accumulator_merge {
        *reducing_chunk_id_counter += 1;
        let chunk_id = *reducing_chunk_id_counter;

        let new_chunk = TargetKeyChunk::Reducing(TargetKeyReducingChunk { chunk_id });

        (chunk_id, new_chunk)
    } else {
        let buffer = free_reduce_eval_action_buffers.take();

        let new_chunk = TargetKeyChunk::Collecting(TargetKeyCollectingChunk {
            accumulator_id: Some(accumulator_id),
            is_accumulator_pending: false,
            is_reducing: true,
            reduce_input: Serializer::from_vec(buffer),
            reduce_input_items: free_serializer_reduce_input_items_buffers.take(),
        });

        (0, new_chunk)
    };

    mem::swap(last_chunk, &mut new_chunk);
    let TargetKeyCollectingChunk {
        accumulator_id: _,
        is_accumulator_pending: _,
        is_reducing: _,
        mut reduce_input,
        reduce_input_items,
    } = new_chunk
        .into_collecting()
        .expect("last chunk should be Collecting");

    let items = reduce_input.create_vector(&reduce_input_items);
    free_serializer_reduce_input_items_buffers.push(reduce_input_items);

    let input = AggregateReduceInput::create(
        reduce_input.buffer_builder(),
        &AggregateReduceInputArgs { items: Some(items) },
    );
    let input = reduce_input.finish(input).into_owned();

    actions.push((
        ActionType::FunctionEval(FunctionEvalAction::AggregateReduce(
            AggregateReduceEvalAction {
                accumulator: accumulator_id,
                target_info: target_info_id,
                input,
            },
        )),
        HandlerContext::Reducing(ReducingContext {
            target_key: target_key_rc,
            chunk_id: new_chunk_id,
        }),
        input_handler!(this, AggregateTransform, ctx, HandlerContext, input, {
            let ctx = ctx.into_reducing().expect("should be ReducingContext");
            let FunctionEvalInput { body } = input.into_eval_aggregate_reduce()?;
            this.on_reduce_received(ctx, body)
        }),
    ));
}
