use diffbelt_types::collection::diff::{
    DiffCollectionRequestJsonData, DiffCollectionResponseJsonData, KeyValueDiffJsonData,
    ReaderDiffFromDefJsonData,
};
use diffbelt_types::collection::generation::StartGenerationRequestJsonData;
use generational_arena::Arena;
use std::borrow::Cow;
use std::mem;
use std::ops::Deref;

use diffbelt_util::cast::usize_to_u64;

use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::{Action, ActionType};
use crate::base::error::TransformError;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::base::input::{Input, InputType};
use crate::TransformRunResult;

enum State {
    Uninitialized,
    AwaitingForGenerationStart(AwaitingForGenerationStartState),
    Processing(ProcessingState),
}

struct AwaitingForGenerationStartState {
    items: Vec<KeyValueDiffJsonData>,
    cursor_id: Option<Box<str>>,
}

struct ProcessingState {
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

        self.state =
            State::AwaitingForGenerationStart(AwaitingForGenerationStartState { items, cursor_id });

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
        // TODO: we need take state first to take ownership on items

        let State::AwaitingForGenerationStart(state) = &self.state else {
            return Err(TransformError::Unspecified(
                "State is not AwaitingForGeneration".to_string(),
            ));
        };

        let mut actions = Vec::<(_, ActionInputHandler)>::new();
        let mut actions_left = state.items.len();

        // TODO: add items actions

        if let Some(cursor_id) = state.cursor_id.as_ref() {
            actions_left += 1;

            actions.push((
                ActionType::DiffbeltCall(DiffbeltCallAction {
                    method: Method::Get,
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

        self.state = State::Processing(ProcessingState {
            actions_left,
            cursor_id: None,
        });

        todo!()
    }

    fn on_next_diff_received(&mut self, diff: DiffCollectionResponseJsonData) -> HandlerResult {
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
