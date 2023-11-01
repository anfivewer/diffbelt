use std::borrow::Cow;
use std::mem;
use std::ops::Deref;

use diffbelt_protos::protos::transform::map_filter::{
    MapFilterInput, MapFilterInputArgs, MapFilterMultiInput, MapFilterMultiInputArgs,
    MapFilterMultiOutput,
};
use diffbelt_protos::{deserialize, OwnedSerialized, Serializer};
use generational_arena::{Arena, Index};

use diffbelt_types::collection::diff::{
    DiffCollectionRequestJsonData, DiffCollectionResponseJsonData, KeyValueDiffJsonData,
    ReaderDiffFromDefJsonData,
};
use diffbelt_types::collection::generation::{
    CommitGenerationRequestJsonData, StartGenerationRequestJsonData,
};
use diffbelt_types::collection::put_many::{PutManyRequestJsonData, PutManyResponseJsonData};
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use diffbelt_types::common::key_value_update::KeyValueUpdateJsonData;
use diffbelt_types::common::reader::UpdateReaderJsonData;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util::option::{cut_layer, lift_result_from_option};
use diffbelt_util_no_std::cast::{u64_to_usize, usize_to_u64};

use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::function_eval::{FunctionEvalAction, MapFilterEvalAction};
use crate::base::action::{Action, ActionType};
use crate::base::error::TransformError;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::base::input::function_eval::{FunctionEvalInput, MapFilterEvalInput};
use crate::base::input::{Input, InputType};
use crate::TransformRunResult;

enum State {
    Uninitialized,
    Initialization,
    AwaitingForGenerationStart(AwaitingForGenerationStartState),
    Processing(ProcessingState),
    Committing,
    Invalid,
}

impl State {
    fn as_mut_processing(&mut self) -> Result<&mut ProcessingState, TransformError> {
        let Self::Processing(state) = self else {
            return Err(TransformError::Unspecified(
                "State is not Processing".to_string(),
            ));
        };

        Ok(state)
    }

    fn into_processing(self) -> Result<ProcessingState, TransformError> {
        let Self::Processing(state) = self else {
            return Err(TransformError::Unspecified(
                "State is not Processing".to_string(),
            ));
        };

        Ok(state)
    }

    fn as_commiting(&self) -> Result<(), TransformError> {
        let Self::Committing = self else {
            return Err(TransformError::Unspecified(
                "State is not Commiting".to_string(),
            ));
        };

        Ok(())
    }
}

struct AwaitingForGenerationStartState {
    items: Vec<KeyValueDiffJsonData>,
    cursor_id: Option<Box<str>>,
    to_generation_id: EncodedGenerationIdJsonData,
}

struct ProcessingState {
    to_generation_id: EncodedGenerationIdJsonData,
    actions_left: usize,
    cursor_id: Option<Box<str>>,

    total_items: usize,
    total_chunks: usize,
}

type HandlerResult = Result<ActionInputHandlerResult, TransformError>;

type ActionInputHandler = fn(&mut MapFilterTransform, InputType) -> HandlerResult;

macro_rules! input_handler {
    ( $this:ident, $input:ident, $body:block ) => {
        {
            fn handle_result($this: &mut MapFilterTransform, $input: InputType) -> HandlerResult $body

            handle_result
        }
    };
}

enum ActionInputHandlerResult {
    Finish,
    Consumed,
    AddActions(Vec<(ActionType, ActionInputHandler)>),
}

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

    action_input_handlers: Arena<ActionInputHandler>,
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
            action_input_handlers: Arena::new(),
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

                let ActionInputHandlerResult::AddActions(new_actions) = self.run_init()? else {
                    return Err(TransformError::Unspecified(
                        "Impossible: run_init() is not AddActions".to_string(),
                    ));
                };

                let mut actions = Vec::with_capacity(new_actions.len());

                for (action, handler) in new_actions {
                    self.push_action(&mut actions, action, handler);
                }

                return Ok(TransformRunResult::Actions(actions));
            }
            State::Invalid => {
                return Err(TransformError::Unspecified("State is Invalid".to_string()));
            }
            _ => {}
        }

        let mut must_finish = false;

        let mut actions = Vec::new();

        for input in inputs {
            if must_finish {
                return Err(TransformError::Unspecified(
                    "Expected to finish, but got more inputs".to_string(),
                ));
            }

            let Input { id: (a, b), input } = input;

            let handler = self
                .action_input_handlers
                .remove(Index::from_raw_parts(u64_to_usize(a), b));

            let Some(handler) = handler else {
                return Err(TransformError::Unspecified(
                    "No such handler exists".to_string(),
                ));
            };

            let action_result = handler(self, input)?;

            match action_result {
                ActionInputHandlerResult::Finish => {
                    must_finish = true;
                }
                ActionInputHandlerResult::Consumed => {
                    // Nothing to do, just wait more inputs
                }
                ActionInputHandlerResult::AddActions(new_actions) => {
                    for (action, handler) in new_actions {
                        self.push_action(&mut actions, action, handler);
                    }
                }
            }
        }

        if must_finish {
            if !actions.is_empty() {
                return Err(TransformError::Unspecified(
                    "Expected to finish, but got spawned more actions".to_string(),
                ));
            }

            return Ok(TransformRunResult::Finish);
        }

        Ok(TransformRunResult::Actions(actions))
    }

    fn run_init(&mut self) -> HandlerResult {
        self.state = State::Initialization;

        let mut actions = Vec::<(_, ActionInputHandler)>::with_capacity(1);

        actions.push((
            ActionType::DiffbeltCall(DiffbeltCallAction {
                method: Method::Post,
                path: Cow::Owned(format!(
                    "/collections/{}/diff/",
                    urlencoding::encode(self.from_collection_name.deref())
                )),
                query: Vec::with_capacity(0),
                body: DiffbeltRequestBody::DiffCollectionStart(DiffCollectionRequestJsonData {
                    from_generation_id: None,
                    to_generation_id: None,
                    from_reader: Some(ReaderDiffFromDefJsonData {
                        reader_name: self.reader_name.to_string(),
                        collection_name: Some(self.to_collection_name.to_string()),
                    }),
                }),
            }),
            input_handler!(this, input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                this.on_start_diff(body)
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn on_start_diff(&mut self, diff: DiffCollectionResponseJsonData) -> HandlerResult {
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

        let mut actions = Vec::<(_, ActionInputHandler)>::new();

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
            input_handler!(this, input, {
                let DiffbeltCallInput { body: () } = input.into_diffbelt_ok()?;

                this.on_generation_started()
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn on_generation_started(&mut self) -> HandlerResult {
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

        let mut state = ProcessingState {
            actions_left: 0,
            cursor_id: None,
            to_generation_id,
            total_items: 0,
            total_chunks: 0,
        };

        let mut actions = Vec::new();

        if let Some(cursor_id) = cursor_id.as_ref() {
            state.actions_left += 1;
            actions.push(Self::read_cursor(
                self.from_collection_name.deref(),
                cursor_id.deref(),
            ));
        }

        () = Self::diff_items_to_actions(
            &mut self.free_buffers_for_eval_inputs,
            &mut self.free_buffers_for_eval_outputs,
            &mut state,
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
        state: &mut ProcessingState,
        actions: &mut Vec<(ActionType, ActionInputHandler)>,
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
            state,
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
        state: &mut ProcessingState,
        actions: &mut Vec<(ActionType, ActionInputHandler)>,
        items: Vec<KeyValueDiffJsonData>,
    ) -> Result<(), TransformError> {
        let mut serializer =
            Serializer::from_vec(buffer_for_eval_inputs.take().unwrap_or_else(|| Vec::new()));

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

        let OwnedSerialized { buffer, head, len } =
            serializer.finish(map_filter_multi_input).into_owned();

        state.actions_left += 1;
        actions.push((
            ActionType::FunctionEval(FunctionEvalAction::MapFilter(MapFilterEvalAction {
                inputs_buffer: buffer,
                inputs_head: head,
                inputs_len: len,
                outputs_buffer: buffer_for_eval_outputs.take().unwrap_or_else(|| Vec::new()),
            })),
            input_handler!(this, input, {
                let FunctionEvalInput { body } = input.into_eval_map_filter()?;
                this.on_map_filter_eval_received(body)
            }),
        ));

        Ok(())
    }

    fn on_next_diff_received(&mut self, diff: DiffCollectionResponseJsonData) -> HandlerResult {
        let state = self.state.as_mut_processing()?;

        state.actions_left -= 1;

        let DiffCollectionResponseJsonData {
            from_generation_id: _,
            to_generation_id: _,
            items,
            cursor_id,
        } = diff;

        state.total_items += items.len();
        state.total_chunks += 1;

        let mut actions = Vec::new();

        if items.len() <= self.puts_buffer.len() {
            // Request more items
            if let Some(cursor_id) = cursor_id {
                state.actions_left += 1;
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
            state,
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

    fn on_map_filter_eval_received(&mut self, input: MapFilterEvalInput) -> HandlerResult {
        let state = self.state.as_mut_processing()?;

        state.actions_left -= 1;

        let MapFilterEvalInput {
            inputs_buffer,
            inputs_head,
            inputs_len,
            outputs_buffer,
        } = input;

        self.free_buffers_for_eval_inputs.push(outputs_buffer);

        let bytes = &inputs_buffer[inputs_head..(inputs_head + inputs_len)];
        let map_filter_multi_output =
            deserialize::<MapFilterMultiOutput>(bytes).map_err(NoStdErrorWrap)?;

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

        self.free_buffers_for_eval_outputs.push(inputs_buffer);

        self.post_handle()
    }

    fn post_handle(&mut self) -> HandlerResult {
        let state = self.state.as_mut_processing()?;

        if state.cursor_id.is_some() {
            let avg_items_per_chunk = state.total_items / state.total_chunks;

            let mut actions = Vec::with_capacity(1);

            // Fetch more items or send available
            return if self.puts_buffer.len() < avg_items_per_chunk {
                let cursor_id = state.cursor_id.take().unwrap();

                state.actions_left += 1;
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

        if state.actions_left > 0 {
            return Ok(ActionInputHandlerResult::Consumed);
        }

        let mut actions = Vec::with_capacity(1);

        // Request more items if cursor present
        let cursor_id = state.cursor_id.take();
        if let Some(cursor_id) = cursor_id {
            state.actions_left += 1;
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
            input_handler!(this, input, {
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
        actions: &mut Vec<(ActionType, ActionInputHandler)>,
        new_capacity: usize,
    ) {
        if puts_buffer.is_empty() {
            return;
        }

        let mut items = Vec::with_capacity(new_capacity);
        mem::swap(&mut items, puts_buffer);

        state.actions_left += 1;
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
            input_handler!(this, input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_put_many()?;
                let PutManyResponseJsonData { generation_id: _ } = body;

                let state = this.state.as_mut_processing()?;

                state.actions_left -= 1;

                this.post_handle()
            }),
        ));
    }

    fn read_cursor(
        from_collection_name: &str,
        cursor_id: &str,
    ) -> (ActionType, ActionInputHandler) {
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
            input_handler!(this, input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                this.on_next_diff_received(body)
            }),
        )
    }

    fn push_action(
        &mut self,
        actions: &mut Vec<Action>,
        action: ActionType,
        handler: ActionInputHandler,
    ) {
        let index = self.action_input_handlers.insert(handler);
        let (a, b) = index.into_raw_parts();

        actions.push(Action {
            id: (usize_to_u64(a), b),
            action,
        });
    }
}
