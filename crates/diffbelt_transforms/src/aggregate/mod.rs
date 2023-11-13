use crate::aggregate::context::HandlerContext;
use crate::aggregate::limits::Limits;
use diffbelt_util_no_std::buffers_pool::BuffersPool;

use crate::aggregate::state::State;
use crate::base::error::TransformError;
use crate::base::input::Input;
use crate::transform::{TransformInputs, WithTransformInputs};
use crate::TransformRunResult;

mod context;
mod init;
mod limits;
mod on_diff_received;
mod on_map_received;
mod read_diff_cursor;
mod state;

pub struct AggregateTransform {
    from_collection_name: Box<str>,
    to_collection_name: Box<str>,
    reader_name: Box<str>,
    state: State,
    action_input_handlers: TransformInputs<Self, HandlerContext>,
    max_limits: Limits,
    free_map_eval_action_buffers: BuffersPool<Vec<u8>>,
    free_map_eval_input_buffers: BuffersPool<Vec<u8>>,
}

impl WithTransformInputs<HandlerContext> for AggregateTransform {
    fn transform_inputs_mut(&mut self) -> &mut TransformInputs<Self, HandlerContext> {
        &mut self.action_input_handlers
    }
}

const MB_64: usize = 64 * 1024 * 1024;

impl AggregateTransform {
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
            action_input_handlers: TransformInputs::new(),
            max_limits: Limits {
                pending_eval_map_bytes: MB_64,
                target_data_bytes: 2 * MB_64,
            },
            free_map_eval_action_buffers: BuffersPool::with_capacity(4),
            free_map_eval_input_buffers: BuffersPool::with_capacity(4),
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

                let new_actions = self.run_init();

                let mut actions = Vec::with_capacity(new_actions.len());

                for (action, ctx, handler) in new_actions {
                    self.action_input_handlers
                        .push_action(&mut actions, action, ctx, handler);
                }

                return Ok(TransformRunResult::Actions(actions));
            }
            State::Invalid => {
                return Err(TransformError::Unspecified("State is Invalid".to_string()));
            }
            _ => {}
        }

        match TransformInputs::run(self, inputs) {
            Ok(x) => Ok(x),
            Err(err) => {
                self.state = State::Invalid;
                Err(err)
            }
        }
    }
}
