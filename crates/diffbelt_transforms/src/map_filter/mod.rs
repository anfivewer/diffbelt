use std::borrow::Cow;
use std::mem;
use std::ops::Deref;

use diffbelt_protos::protos::transform::map_filter::{
    MapFilterInput, MapFilterInputArgs, MapFilterMultiInput, MapFilterMultiInputArgs,
};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::diff::{DiffCollectionResponseJsonData, KeyValueDiffJsonData};
use diffbelt_types::collection::generation::{
    CommitGenerationRequestJsonData, StartGenerationRequestJsonData,
};
use diffbelt_types::collection::put_many::{PutManyRequestJsonData, PutManyResponseJsonData};
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use diffbelt_types::common::key_value_update::KeyValueUpdateJsonData;
use diffbelt_types::common::reader::UpdateReaderJsonData;
use diffbelt_util::option::{cut_layer, lift_result_from_option};

use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::function_eval::{FunctionEvalAction, MapFilterEvalAction};
use crate::base::action::{Action, ActionType};
use crate::base::error::TransformError;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::base::input::function_eval::{FunctionEvalInput, MapFilterEvalInput};
use crate::base::input::Input;
use crate::map_filter::state::{AwaitingForGenerationStartState, ProcessingState, State};
use crate::transform::{
    ActionInputHandlerAction, ActionInputHandlerActionsVec, ActionInputHandlerResult,
    HandlerResult, TransformInputs, WithTransformInputs,
};
use crate::{input_handler, TransformRunResult};

mod state;

pub struct MapFilterTransform {
    from_collection_name: Box<str>,
    to_collection_name: Box<str>,
    reader_name: Box<str>,
    state: State,

    puts_buffer: Vec<KeyValueUpdateJsonData>,
    /// for `MapFilterMultiOutput`
    free_buffers_for_eval_inputs: Vec<Vec<u8>>,
    /// for `MapFilterMultiInput`
    free_buffers_for_eval_outputs: Vec<Vec<u8>>,

    action_input_handlers: TransformInputs<Self, ()>,
}

impl WithTransformInputs<()> for MapFilterTransform {
    fn transform_inputs_mut(&mut self) -> &mut TransformInputs<Self, ()> {
        &mut self.action_input_handlers
    }
}

impl MapFilterTransform {
    pub fn new(
        from_collection_name: Box<str>,
        to_collection_name: Box<str>,
        reader_name: Box<str>,
    ) -> Self {
        Self {
            from_collection_name,
            to_collection_name,
            reader_name,
            state: State::Uninitialized,
            puts_buffer: Vec::new(),
            free_buffers_for_eval_inputs: Vec::with_capacity(4),
            free_buffers_for_eval_outputs: Vec::with_capacity(4),
            action_input_handlers: TransformInputs::new(),
        }
    }

    pub fn run(&mut self, inputs: Vec<Input>) -> Result<TransformRunResult, TransformError> {
        match self.state {
            State::Uninitialized => {
                if !inputs.is_empty() {
                    return Err(TransformError::Unspecified(
                        "Unexpected inputs on init".to_string(),
                    ));
                }

                let ActionInputHandlerResult::AddActions(mut new_actions) = self.run_init()? else {
                    return Err(TransformError::Unspecified(
                        "Impossible: run_init() is not AddActions".to_string(),
                    ));
                };

                let mut actions = self.action_input_handlers.take_actions_vec();

                for (action, (), handler) in new_actions.drain(..) {
                    self.action_input_handlers
                        .push_action(&mut actions, action, (), handler);
                }

                self.action_input_handlers
                    .return_action_input_actions_vec(new_actions);

                return Ok(TransformRunResult::Actions(actions));
            }
            State::Invalid => {
                return Err(TransformError::Unspecified("State is Invalid".to_string()));
            }
            _ => {}
        }

        TransformInputs::run(self, inputs)
    }

    pub fn return_actions_vec(&mut self, buffer: Vec<Action>) {
        self.action_input_handlers.return_actions_vec(buffer);
    }

    fn run_init(&mut self) -> HandlerResult<Self, ()> {
        self.state = State::Initialization;

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        actions.push((
            ActionType::new_diff_call_by_reader(
                self.from_collection_name.deref(),
                self.reader_name.deref(),
                self.to_collection_name.deref(),
            ),
            (),
            input_handler!(this, MapFilterTransform, _ctx, (), input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                this.on_start_diff(body)
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn on_start_diff(&mut self, diff: DiffCollectionResponseJsonData) -> HandlerResult<Self, ()> {
        let DiffCollectionResponseJsonData {
            from_generation_id,
            to_generation_id,
            items,
            cursor_id,
        } = diff;

        if from_generation_id == to_generation_id {
            self.state = State::Invalid;
            return Ok(ActionInputHandlerResult::Finish);
        }

        self.state = State::AwaitingForGenerationStart(AwaitingForGenerationStartState {
            items,
            cursor_id,
            to_generation_id: to_generation_id.clone(),
        });

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        actions.push((
            ActionType::DiffbeltCall(DiffbeltCallAction {
                method: Method::Post,
                path: Cow::Owned(format!(
                    "/collections/{}/generation/start",
                    urlencoding::encode(self.to_collection_name.deref())
                )),
                query: Vec::with_capacity(0),
                body: DiffbeltRequestBody::StartGeneration(StartGenerationRequestJsonData {
                    generation_id: to_generation_id,
                    abort_outdated: Some(true),
                }),
            }),
            (),
            input_handler!(this, MapFilterTransform, _ctx, (), input, {
                let DiffbeltCallInput { body: () } = input.into_diffbelt_ok()?;

                this.on_generation_started()
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn on_generation_started(&mut self) -> HandlerResult<Self, ()> {
        let mut old_state = State::Invalid;
        mem::swap(&mut old_state, &mut self.state);

        let State::AwaitingForGenerationStart(old_state) = old_state else {
            return Err(TransformError::Unspecified(
                "State is not AwaitingForGeneration".to_string(),
            ));
        };

        let AwaitingForGenerationStartState {
            items,
            cursor_id,
            to_generation_id,
        } = old_state;

        let state = ProcessingState {
            cursor_id: None,
            to_generation_id,
            total_items: 0,
            total_chunks: 0,
        };

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        if let Some(cursor_id) = cursor_id.as_ref() {
            actions.push(Self::read_cursor(
                self.from_collection_name.deref(),
                cursor_id.deref(),
            ));
        }

        () = Self::diff_items_to_actions(
            &mut self.free_buffers_for_eval_inputs,
            &mut self.free_buffers_for_eval_outputs,
            &mut actions,
            items,
        )?;

        self.state = State::Processing(state);

        if actions.is_empty() {
            // If no actions added it means that we have no items,
            // no cursor, so we can just commit generation
            return self.post_handle();
        }

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn diff_items_to_actions(
        free_buffers_for_eval_inputs: &mut Vec<Vec<u8>>,
        free_buffers_for_eval_outputs: &mut Vec<Vec<u8>>,
        actions: &mut ActionInputHandlerActionsVec<Self, ()>,
        items: Vec<KeyValueDiffJsonData>,
    ) -> Result<(), TransformError> {
        if items.is_empty() {
            return Ok(());
        }

        let mut buffer_for_eval_inputs = free_buffers_for_eval_inputs.pop().map(|mut x| {
            x.clear();
            x
        });
        let mut buffer_for_eval_outputs = free_buffers_for_eval_outputs.pop().map(|mut x| {
            x.clear();
            x
        });

        let result = Self::diff_items_to_actions_inner(
            &mut buffer_for_eval_inputs,
            &mut buffer_for_eval_outputs,
            actions,
            items,
        );

        match result {
            Ok(x) => Ok(x),
            Err(err) => {
                if let Some(buffer) = buffer_for_eval_inputs {
                    free_buffers_for_eval_inputs.push(buffer);
                }
                if let Some(buffer) = buffer_for_eval_outputs {
                    free_buffers_for_eval_outputs.push(buffer);
                }

                Err(err)
            }
        }
    }

    fn diff_items_to_actions_inner(
        buffer_for_eval_inputs: &mut Option<Vec<u8>>,
        buffer_for_eval_outputs: &mut Option<Vec<u8>>,
        actions: &mut ActionInputHandlerActionsVec<Self, ()>,
        items: Vec<KeyValueDiffJsonData>,
    ) -> Result<(), TransformError> {
        let mut serializer = Serializer::<MapFilterMultiInput>::from_vec(
            buffer_for_eval_inputs.take().unwrap_or_else(|| Vec::new()),
        );

        macro_rules! ok {
            ($expr:expr) => {
                match $expr {
                    Ok(x) => x,
                    Err(err) => {
                        let buffer = serializer.into_vec();
                        buffer_for_eval_inputs.replace(buffer);
                        return Err(err.into());
                    }
                }
            };
        }

        let mut records = Vec::with_capacity(items.len());

        for item in items {
            let KeyValueDiffJsonData {
                key,
                from_value,
                intermediate_values: _,
                to_value,
            } = item;

            let key = ok!(key.into_bytes());

            let from_value = cut_layer(from_value).map(|x| x.into_bytes());
            let from_value = ok!(lift_result_from_option(from_value));

            let to_value = cut_layer(to_value).map(|x| x.into_bytes());
            let to_value = ok!(lift_result_from_option(to_value));

            let source_key = serializer.create_vector(&key);
            let source_old_value = from_value.map(|x| serializer.create_vector(&x));
            let source_new_value = to_value.map(|x| serializer.create_vector(&x));

            records.push(MapFilterInput::create(
                serializer.buffer_builder(),
                &MapFilterInputArgs {
                    source_key: Some(source_key),
                    source_old_value,
                    source_new_value,
                },
            ));
        }

        let records = serializer.create_vector(&records);
        let map_filter_multi_input = MapFilterMultiInput::create(
            serializer.buffer_builder(),
            &MapFilterMultiInputArgs {
                items: Some(records),
            },
        );

        let input = serializer.finish(map_filter_multi_input).into_owned();

        actions.push((
            ActionType::FunctionEval(FunctionEvalAction::MapFilter(MapFilterEvalAction {
                input,
                output_buffer: buffer_for_eval_outputs.take().unwrap_or_else(|| Vec::new()),
            })),
            (),
            input_handler!(this, MapFilterTransform, _ctx, (), input, {
                let FunctionEvalInput { body } = input.into_eval_map_filter()?;
                this.on_map_filter_eval_received(body)
            }),
        ));

        Ok(())
    }

    fn on_next_diff_received(
        &mut self,
        diff: DiffCollectionResponseJsonData,
    ) -> HandlerResult<Self, ()> {
        let state = self.state.as_mut_processing()?;

        let DiffCollectionResponseJsonData {
            from_generation_id: _,
            to_generation_id: _,
            items,
            cursor_id,
        } = diff;

        state.total_items += items.len();
        state.total_chunks += 1;

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        if items.len() <= self.puts_buffer.len() {
            // Request more items
            if let Some(cursor_id) = cursor_id {
                actions.push(Self::read_cursor(
                    self.from_collection_name.deref(),
                    cursor_id.deref(),
                ));
            }
        } else {
            state.cursor_id = cursor_id;
        }

        () = Self::diff_items_to_actions(
            &mut self.free_buffers_for_eval_inputs,
            &mut self.free_buffers_for_eval_outputs,
            &mut actions,
            items,
        )?;

        let avg_items_per_chunk = state.total_items / state.total_chunks;
        if self.puts_buffer.len() >= avg_items_per_chunk {
            let new_capacity = self.puts_buffer.capacity();
            Self::flush_puts(
                self.to_collection_name.deref(),
                state,
                &mut self.puts_buffer,
                &mut actions,
                new_capacity,
            );
        }

        if actions.is_empty() {
            // If no actions added it means that we have no items,
            // then check for cursor and maybe commit generation
            return self.post_handle();
        }

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn on_map_filter_eval_received(
        &mut self,
        input: MapFilterEvalInput,
    ) -> HandlerResult<Self, ()> {
        let MapFilterEvalInput {
            input,
            action_input_buffer: outputs_buffer,
        } = input;

        self.free_buffers_for_eval_inputs.push(outputs_buffer);

        let map_filter_multi_output = input.data();

        let Some(records) = map_filter_multi_output.target_update_records() else {
            return Err(TransformError::Unspecified(
                "map_filter eval output has None records".to_string(),
            ));
        };

        for record in records {
            let key = record
                .key()
                .ok_or_else(|| {
                    TransformError::Unspecified(
                        "map_filter eval output has record with None key".to_string(),
                    )
                })?
                .bytes();
            let value = record.value().map(|x| x.bytes());

            self.puts_buffer.push(KeyValueUpdateJsonData {
                key: EncodedKeyJsonData::from_bytes_slice(key),
                if_not_present: None,
                value: value.map(|x| EncodedValueJsonData::from_bytes_slice(x)),
            });
        }

        self.free_buffers_for_eval_outputs.push(input.into_vec());

        self.post_handle()
    }

    fn post_handle(&mut self) -> HandlerResult<Self, ()> {
        let state = self.state.as_mut_processing()?;

        if state.cursor_id.is_some() {
            let avg_items_per_chunk = state.total_items / state.total_chunks;

            let mut actions = self.action_input_handlers.take_action_input_actions_vec();

            // Fetch more items or send available
            return if self.puts_buffer.len() < avg_items_per_chunk {
                let cursor_id = state.cursor_id.take().unwrap();

                actions.push(Self::read_cursor(
                    self.from_collection_name.deref(),
                    cursor_id.deref(),
                ));

                Ok(ActionInputHandlerResult::AddActions(actions))
            } else {
                let new_capacity = self.puts_buffer.capacity();

                Self::flush_puts(
                    self.to_collection_name.deref(),
                    state,
                    &mut self.puts_buffer,
                    &mut actions,
                    new_capacity,
                );

                Ok(ActionInputHandlerResult::AddActions(actions))
            };
        }

        if self.action_input_handlers.has_pending_actions() {
            return Ok(ActionInputHandlerResult::Consumed);
        }

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

        // Request more items if cursor present
        let cursor_id = state.cursor_id.take();
        if let Some(cursor_id) = cursor_id {
            actions.push(Self::read_cursor(
                self.from_collection_name.deref(),
                cursor_id.deref(),
            ));

            return Ok(ActionInputHandlerResult::AddActions(actions));
        }

        // Make rest puts
        if !self.puts_buffer.is_empty() {
            Self::flush_puts(
                self.to_collection_name.deref(),
                state,
                &mut self.puts_buffer,
                &mut actions,
                0,
            );

            return Ok(ActionInputHandlerResult::AddActions(actions));
        }

        let mut state = State::Committing;
        mem::swap(&mut self.state, &mut state);

        let ProcessingState {
            to_generation_id, ..
        } = state.into_processing()?;

        // No need to increment actions_count, since we are replaced state
        actions.push((
            ActionType::DiffbeltCall(DiffbeltCallAction {
                method: Method::Post,
                path: Cow::Owned(format!(
                    "/collections/{}/generation/commit",
                    urlencoding::encode(self.to_collection_name.deref()),
                )),
                query: Vec::with_capacity(0),
                body: DiffbeltRequestBody::CommitGeneration(CommitGenerationRequestJsonData {
                    generation_id: to_generation_id.clone(),
                    update_readers: Some(vec![UpdateReaderJsonData {
                        reader_name: self.reader_name.to_string(),
                        generation_id: to_generation_id,
                    }]),
                }),
            }),
            (),
            input_handler!(this, MapFilterTransform, _ctx, (), input, {
                let DiffbeltCallInput { body: () } = input.into_diffbelt_ok()?;

                () = this.state.as_commiting()?;

                // Not finishing, repeat cycle until we get diff result with from_generation = to_generation
                this.run_init()
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn flush_puts(
        to_collection_name: &str,
        state: &mut ProcessingState,
        puts_buffer: &mut Vec<KeyValueUpdateJsonData>,
        actions: &mut ActionInputHandlerActionsVec<Self, ()>,
        new_capacity: usize,
    ) {
        if puts_buffer.is_empty() {
            return;
        }

        let mut items = Vec::with_capacity(new_capacity);
        mem::swap(&mut items, puts_buffer);

        actions.push((
            ActionType::DiffbeltCall(DiffbeltCallAction {
                method: Method::Post,
                path: Cow::Owned(format!(
                    "/collections/{}/putMany",
                    urlencoding::encode(to_collection_name),
                )),
                query: Vec::with_capacity(0),
                body: DiffbeltRequestBody::PutMany(PutManyRequestJsonData {
                    items,
                    generation_id: Some(state.to_generation_id.clone()),
                    phantom_id: None,
                }),
            }),
            (),
            input_handler!(this, MapFilterTransform, _ctx, (), input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_put_many()?;
                let PutManyResponseJsonData { generation_id: _ } = body;

                this.post_handle()
            }),
        ));
    }

    fn read_cursor(
        from_collection_name: &str,
        cursor_id: &str,
    ) -> ActionInputHandlerAction<Self, ()> {
        (
            ActionType::DiffbeltCall(DiffbeltCallAction {
                method: Method::Get,
                path: Cow::Owned(format!(
                    "/collections/{}/diff/{}",
                    urlencoding::encode(from_collection_name),
                    urlencoding::encode(cursor_id),
                )),
                query: Vec::with_capacity(0),
                body: DiffbeltRequestBody::ReadDiffCursorNone,
            }),
            (),
            input_handler!(this, MapFilterTransform, _ctx, (), input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                this.on_next_diff_received(body)
            }),
        )
    }
}
