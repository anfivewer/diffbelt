use diffbelt_types::collection::diff::{
    DiffCollectionRequestJsonData, DiffCollectionResponseJsonData, KeyValueDiffJsonData,
    ReaderDiffFromDefJsonData,
};
use diffbelt_types::collection::generation::StartGenerationRequestJsonData;
use diffbelt_types::collection::put_many::{PutManyRequestJsonData, PutManyResponseJsonData};
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_types::common::key_value::EncodedKeyJsonData;
use diffbelt_types::common::key_value_update::KeyValueUpdateJsonData;
use generational_arena::Arena;
use std::borrow::Cow;
use std::mem;
use std::ops::Deref;
use std::string::FromUtf8Error;

use diffbelt_util::cast::usize_to_u64;
use diffbelt_util::option::{cut_layer, lift_result_from_option};

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
    AwaitingForGenerationStart(AwaitingForGenerationStartState),
    Processing(ProcessingState),
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
            action_input_handlers: Arena::new(),
        }
    }

    pub fn run(&mut self, inputs: Vec<Input>) -> TransformRunResult {
        todo!()
    }

    fn run_init(&mut self, inputs: Vec<Input>) -> TransformRunResult {
        if !inputs.is_empty() {
            return TransformRunResult::Error(TransformError::Unspecified(
                "Unexpected inputs on init".to_string(),
            ));
        }

        let mut actions = Vec::new();

        self.push_action(
            &mut actions,
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
        );

        TransformRunResult::Actions(actions)
    }

    fn on_start_diff(&mut self, diff: DiffCollectionResponseJsonData) -> HandlerResult {
        let DiffCollectionResponseJsonData {
            from_generation_id: _,
            to_generation_id,
            items,
            cursor_id,
        } = diff;

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
                    urlencoding::encode(self.from_collection_name.deref())
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
        };

        let mut actions = Vec::<(_, ActionInputHandler)>::new();

        for item in items {
            let KeyValueDiffJsonData {
                key,
                from_value,
                intermediate_values: _,
                to_value,
            } = item;

            let key = key.into_bytes()?;

            let from_value = cut_layer(from_value).map(|x| x.into_bytes());
            let from_value = lift_result_from_option(from_value)?;

            let to_value = cut_layer(to_value).map(|x| x.into_bytes());
            let to_value = lift_result_from_option(to_value)?;

            state.actions_left += 1;

            actions.push((
                ActionType::FunctionEval(FunctionEvalAction::MapFilter(MapFilterEvalAction {
                    key,
                    from_value,
                    to_value,
                })),
                input_handler!(this, input, {
                    let FunctionEvalInput { body } = input.into_eval_map_filter()?;

                    this.on_map_filter_eval_received(body)
                }),
            ));
        }

        if let Some(cursor_id) = cursor_id.as_ref() {
            state.actions_left += 1;

            actions.push((
                ActionType::DiffbeltCall(DiffbeltCallAction {
                    method: Method::Post,
                    path: Cow::Owned(format!(
                        "/collections/{}/diff/{}",
                        urlencoding::encode(self.from_collection_name.deref()),
                        urlencoding::encode(cursor_id.deref()),
                    )),
                    query: Vec::with_capacity(0),
                    body: DiffbeltRequestBody::None,
                }),
                input_handler!(this, input, {
                    let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                    this.on_next_diff_received(body)
                }),
            ));
        }

        self.state = State::Processing(state);

        todo!()
    }

    fn on_next_diff_received(&mut self, diff: DiffCollectionResponseJsonData) -> HandlerResult {
        todo!()
    }

    fn on_map_filter_eval_received(&mut self, input: MapFilterEvalInput) -> HandlerResult {
        let mut state = self.state.as_mut_processing()?;

        state.actions_left -= 1;

        let MapFilterEvalInput {
            old_key,
            new_key,
            value,
        } = input;

        let mut actions = Vec::<(_, ActionInputHandler)>::new();

        let (Some(new_key), Some(value)) = (new_key, value) else {
            self.puts_buffer.push(KeyValueUpdateJsonData {
                key: EncodedKeyJsonData::from_boxed_bytes(old_key),
                if_not_present: None,
                value: None,
            });

            return Self::post_handle(
                self.from_collection_name.deref(),
                state,
                &mut self.puts_buffer,
            );
        };

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn post_handle(
        from_collection_name: &str,
        state: &mut ProcessingState,
        puts_buffer: &mut Vec<KeyValueUpdateJsonData>,
    ) -> HandlerResult {
        if state.actions_left > 0 {
            return Ok(ActionInputHandlerResult::Consumed);
        }

        // Make rest puts
        if !puts_buffer.is_empty() {
            state.actions_left += 1;
            let mut actions = Vec::<(_, ActionInputHandler)>::new();

            let mut items = Vec::with_capacity(0);
            mem::swap(&mut items, puts_buffer);

            actions.push((
                ActionType::DiffbeltCall(DiffbeltCallAction {
                    method: Method::Post,
                    path: Cow::Owned(format!(
                        "/collections/{}/putMany",
                        urlencoding::encode(from_collection_name),
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

                    let mut state = this.state.as_mut_processing()?;

                    state.actions_left -= 1;

                    MapFilterTransform::post_handle(
                        this.from_collection_name.deref(),
                        &mut state,
                        &mut this.puts_buffer,
                    )
                }),
            ));

            return Ok(ActionInputHandlerResult::AddActions(actions));
        }

        todo!()
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
