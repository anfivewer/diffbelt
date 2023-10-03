use generational_arena::Arena;

use diffbelt_util::cast::usize_to_u64;

use crate::base::action::{Action, ActionType};
use crate::base::error::TransformError;
use crate::base::input::{Input, InputType};
use crate::TransformRunResult;

enum State {
    Uninitialized,
}

type ActionInputHandler = Box<
    dyn Fn(&mut MapFilterTransform, InputType) -> Result<ActionInputHandlerResult, TransformError>,
>;

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

        todo!("Call diff method")
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
