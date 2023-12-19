use diffbelt_util_no_std::buffers_pool::BuffersPool;
use diffbelt_util_no_std::temporary_collection::immutable::hash_set::TemporaryRefHashSet;
use diffbelt_util_no_std::temporary_collection::mutable::vec::TemporaryMutRefVec;
pub use state::AggregateTransform;

use crate::aggregate::context::HandlerContext;
use crate::aggregate::limits::Limits;
use crate::aggregate::state::State;
use crate::base::action::Action;
use crate::base::common::accumulator::AccumulatorId;
use crate::base::error::TransformError;
use crate::base::input::Input;
use crate::transform::{TransformInputs, WithTransformInputs};
use crate::TransformRunResult;

mod apply;
mod context;
#[cfg(test)]
mod debug_print;
mod init;
mod limits;
mod merge;
mod on_diff_received;
mod on_initial_accumulator_received;
mod on_map_received;
mod on_merge_received;
mod on_reduce_received;
mod on_target_info_received;
mod on_target_record_received;
mod read_diff_cursor;
mod state;

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
        supports_accumulator_merge: bool,
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
                ..Default::default()
            },
            supports_accumulator_merge,
            updated_target_keys_temp_set: TemporaryRefHashSet::new(),
            apply_target_keys_temp_vec: TemporaryMutRefVec::new(),
            free_map_eval_action_buffers: BuffersPool::with_capacity(4),
            free_map_eval_input_buffers: BuffersPool::with_capacity(4),
            free_target_info_action_buffers: BuffersPool::with_capacity(4),
            free_reduce_eval_action_buffers: BuffersPool::with_capacity(4),
            free_serializer_reduce_input_items_buffers: BuffersPool::with_capacity(4),
            free_merge_accumulator_ids_vecs: BuffersPool::with_capacity(4),
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

                let mut new_actions = self.run_init();

                let mut actions = self.action_input_handlers.take_actions_vec();

                for (action, ctx, handler) in new_actions.drain(..) {
                    self.action_input_handlers
                        .push_action(&mut actions, action, ctx, handler);
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

        match TransformInputs::run(self, inputs) {
            Ok(x) => Ok(x),
            Err(err) => {
                self.state = State::Invalid;
                Err(err)
            }
        }
    }

    pub fn return_target_info_action_buffer(&mut self, buffer: Vec<u8>) {
        self.free_target_info_action_buffers.push(buffer);
    }

    pub fn return_actions_vec(&mut self, buffer: Vec<Action>) {
        self.action_input_handlers.return_actions_vec(buffer);
    }

    pub fn return_merge_accumulator_ids_vec(&mut self, buffer: Vec<AccumulatorId>) {
        self.free_merge_accumulator_ids_vecs.push(buffer);
    }
}
