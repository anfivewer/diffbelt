use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::ActionType;
use crate::input_handler;
use crate::transform::ActionInputHandlerAction;
use std::borrow::Cow;
use crate::base::input::diffbelt_call::DiffbeltCallInput;

impl AggregateTransform {
    pub fn read_cursor(from_collection_name: &str, cursor_id: &str) -> ActionInputHandlerAction<Self> {
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
            input_handler!(this, AggregateTransform, input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                this.on_diff_received(body)
            }),
        )
    }
}
