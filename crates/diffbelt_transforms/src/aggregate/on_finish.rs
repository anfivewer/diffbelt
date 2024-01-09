use crate::aggregate::context::HandlerContext;
use crate::aggregate::state::State;
use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::ActionType;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::input_handler;
use crate::transform::{ActionInputHandlerResult, HandlerResult};
use diffbelt_types::collection::generation::CommitGenerationRequestJsonData;
use diffbelt_types::common::reader::UpdateReaderJsonData;
use std::borrow::Cow;
use std::mem;
use std::ops::Deref;

impl AggregateTransform {
    pub fn on_finish(&mut self) -> HandlerResult<Self, HandlerContext> {
        let state = self.state.expect_processing_mut()?;

        if self.action_input_handlers.has_pending_actions() {
            return Ok(ActionInputHandlerResult::Consumed);
        }

        assert!(
            state.current_limits.is_empty(),
            "No pending actions but limits are not empty"
        );

        let mut state = State::AwaitingCommitGeneration;

        mem::swap(&mut self.state, &mut state);

        let State::Processing(state) = state else {
            panic!("already checked");
        };

        let to_generation_id = state.to_generation_id;

        let mut actions = self.action_input_handlers.take_action_input_actions_vec();

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
            HandlerContext::None,
            input_handler!(this, AggregateTransform, _ctx, HandlerContext, input, {
                let DiffbeltCallInput { body: () } = input.into_diffbelt_ok()?;

                this.on_commit_generation()
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    pub fn on_commit_generation(&mut self) -> HandlerResult<Self, HandlerContext> {
        Ok(ActionInputHandlerResult::AddActions(self.run_init()))
    }
}
