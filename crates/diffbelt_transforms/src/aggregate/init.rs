use std::borrow::Cow;
use std::mem;
use std::ops::Deref;

use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::collection::generation::StartGenerationRequestJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;

use crate::aggregate::state::{ProcessingState, State};
use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::ActionType;
use crate::base::error::TransformError;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::input_handler;
use crate::transform::{ActionInputHandlerActionsVec, ActionInputHandlerResult, HandlerResult};

impl AggregateTransform {
    pub fn run_init(&mut self) -> ActionInputHandlerActionsVec<Self> {
        self.state = State::AwaitingDiff;

        let mut actions = ActionInputHandlerActionsVec::with_capacity(1);

        actions.push((
            ActionType::new_diff_call_by_reader(
                self.from_collection_name.deref(),
                self.reader_name.deref(),
                self.to_collection_name.deref(),
            ),
            input_handler!(this, AggregateTransform, input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                this.on_start_diff(body)
            }),
        ));

        actions
    }

    fn on_start_diff(&mut self, diff: DiffCollectionResponseJsonData) -> HandlerResult<Self> {
        let DiffCollectionResponseJsonData {
            from_generation_id,
            to_generation_id,
            ..
        } = &diff;

        if from_generation_id == to_generation_id {
            self.state = State::Invalid;
            return Ok(ActionInputHandlerResult::Finish);
        }

        let to_generation_id = to_generation_id.clone();

        self.state = State::AwaitingGenerationStart { diff };

        let mut actions = ActionInputHandlerActionsVec::with_capacity(1);

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
            input_handler!(this, AggregateTransform, input, {
                let DiffbeltCallInput { body: () } = input.into_diffbelt_ok()?;

                this.on_generation_started()
            }),
        ));

        Ok(ActionInputHandlerResult::AddActions(actions))
    }

    fn on_generation_started(&mut self) -> HandlerResult<Self> {
        let mut old_state = State::Invalid;
        mem::swap(&mut old_state, &mut self.state);

        let State::AwaitingGenerationStart { diff } = old_state else {
            return Err(TransformError::Unspecified(
                "State is not AwaitingForGeneration".to_string(),
            ));
        };

        let state = ProcessingState {
            cursor_id: None,
            to_generation_id: diff.to_generation_id.clone(),
            pending_eval_bytes: 0,
        };

        self.state = State::Processing(state);

        self.on_diff_received(diff)
    }
}
