mod r#trait;

use crate::base::action::{Action, ActionType};
use crate::base::error::TransformError;
use crate::base::input::{Input, InputType};
use crate::TransformRunResult;
use diffbelt_util_no_std::buffers_pool::BuffersPool;
use diffbelt_util_no_std::cast::{u64_to_usize, usize_to_u64};
use generational_arena::{Arena, Index};

pub use r#trait::Transform;

pub type HandlerResult<This, Context> =
    Result<ActionInputHandlerResult<This, Context>, TransformError>;

pub type ActionInputHandler<This, Context> =
    fn(&mut This, ctx: Context, InputType) -> HandlerResult<This, Context>;

pub type ActionInputHandlerAction<This, Context> =
    (ActionType, Context, ActionInputHandler<This, Context>);

pub type ActionInputHandlerActionsVec<This, Context> = Vec<ActionInputHandlerAction<This, Context>>;

pub enum ActionInputHandlerResult<This, Context> {
    Finish,
    Consumed,
    AddActions(ActionInputHandlerActionsVec<This, Context>),
}

#[macro_export]
macro_rules! input_handler {
    ($this:ident, $this_type:ty, $ctx:ident, $ctx_type:ty, $input:ident, $body:block) => {
        {
            fn handle_result($this: &mut $this_type, $ctx: $ctx_type, $input: crate::base::input::InputType) -> crate::transform::HandlerResult<$this_type, $ctx_type> $body

            handle_result
        }
    };
}

pub trait WithTransformInputs<Context>: Sized {
    fn transform_inputs_mut(&mut self) -> &mut TransformInputs<Self, Context>;
}

pub struct TransformInputs<This, Context> {
    actions_left: usize,
    action_input_actions_buffers: BuffersPool<ActionInputHandlerActionsVec<This, Context>>,
    actions_buffers: BuffersPool<Vec<Action>>,
    arena: Arena<(Context, ActionInputHandler<This, Context>)>,
}

impl<Context, This: WithTransformInputs<Context>> TransformInputs<This, Context> {
    pub fn new() -> Self {
        Self {
            actions_left: 0,
            action_input_actions_buffers: BuffersPool::with_capacity(4),
            actions_buffers: BuffersPool::with_capacity(4),
            arena: Arena::new(),
        }
    }

    pub fn run(this: &mut This, inputs: &mut Vec<Input>) -> Result<TransformRunResult, TransformError> {
        let mut must_finish = false;

        let mut actions = { this.transform_inputs_mut().actions_buffers.take() };

        for input in inputs.drain(..) {
            if must_finish {
                return Err(TransformError::Unspecified(
                    "Expected to finish, but got more inputs".to_string(),
                ));
            }

            let Input { id: (a, b), input } = input;

            let (ctx, handler) = {
                let transform_inputs = this.transform_inputs_mut();

                if transform_inputs.actions_left == 0 {
                    return Err(TransformError::Unspecified(
                        "actions_left == 0, but input received".to_string(),
                    ));
                }

                let handler = transform_inputs
                    .arena
                    .remove(Index::from_raw_parts(u64_to_usize(a), b));

                let Some(ctx_and_handler) = handler else {
                    return Err(TransformError::Unspecified(
                        "No such handler exists".to_string(),
                    ));
                };

                transform_inputs.actions_left -= 1;

                ctx_and_handler
            };

            let action_result = handler(this, ctx, input)?;

            match action_result {
                ActionInputHandlerResult::Finish => {
                    must_finish = true;
                }
                ActionInputHandlerResult::Consumed => {
                    // Nothing to do, just wait more inputs
                }
                ActionInputHandlerResult::AddActions(mut new_actions) => {
                    let transform_inputs = this.transform_inputs_mut();

                    for (action, ctx, handler) in new_actions.drain(..) {
                        transform_inputs.push_action(&mut actions, action, ctx, handler);
                    }

                    transform_inputs
                        .action_input_actions_buffers
                        .push(new_actions);
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

    pub fn take_action_input_actions_vec(&mut self) -> ActionInputHandlerActionsVec<This, Context> {
        self.action_input_actions_buffers.take()
    }

    pub fn return_action_input_actions_vec(
        &mut self,
        buffer: ActionInputHandlerActionsVec<This, Context>,
    ) {
        self.action_input_actions_buffers.push(buffer);
    }

    pub fn take_actions_vec(&mut self) -> Vec<Action> {
        self.actions_buffers.take()
    }

    pub fn return_actions_vec(&mut self, buffer: Vec<Action>) {
        self.actions_buffers.push(buffer);
    }

    pub fn has_pending_actions(&self) -> bool {
        self.actions_left > 0
    }

    pub fn push_action(
        &mut self,
        actions: &mut Vec<Action>,
        action: ActionType,
        ctx: Context,
        handler: ActionInputHandler<This, Context>,
    ) {
        let index = self.arena.insert((ctx, handler));
        let (a, b) = index.into_raw_parts();

        self.actions_left += 1;
        actions.push(Action {
            id: (usize_to_u64(a), b),
            action,
        });
    }
}
