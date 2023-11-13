use crate::aggregate::context::HandlerContext;
use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::ActionType;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::input_handler;
use crate::transform::ActionInputHandlerAction;
use std::borrow::Cow;

impl AggregateTransform {
    pub fn read_cursor(
        from_collection_name: &str,
        cursor_id: &str,
    ) -> ActionInputHandlerAction<Self, HandlerContext> {
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
            HandlerContext::None,
            input_handler!(this, AggregateTransform, _ctx, HandlerContext, input, {
                let DiffbeltCallInput { body } = input.into_diffbelt_diff()?;

                this.on_diff_received(body)
            }),
        )
    }
}
