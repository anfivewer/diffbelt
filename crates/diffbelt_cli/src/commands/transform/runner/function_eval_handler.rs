use std::future::Future;
use std::sync::{Arc, Mutex};

use enum_dispatch::enum_dispatch;

use diffbelt_transforms::base::action::function_eval::FunctionEvalAction;
use diffbelt_transforms::base::input::function_eval::{FunctionEvalInput, FunctionEvalInputBody};
use diffbelt_transforms::Transform;

use crate::commands::errors::TransformEvalError;
use crate::commands::transform::runner::aggregate_eval::AggregateEvalHandler;
use crate::commands::transform::runner::map_filter_eval::MapFilterEvalHandler;
use crate::commands::transform::runner::InputEmitter;

#[enum_dispatch]
pub trait FunctionEvalHandler {
    async fn handle_action(
        &self,
        action: FunctionEvalAction,
        input_emitter: InputEmitter,
        transform: Arc<Mutex<impl Transform>>,
    );
}

#[enum_dispatch(FunctionEvalHandler)]
pub enum FunctionEvalHandlerImpl {
    MapFilter(MapFilterEvalHandler),
    Aggregate(AggregateEvalHandler),
}
