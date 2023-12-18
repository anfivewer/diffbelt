use crate::aggregate::context::HandlerContext;
use crate::aggregate::limits::Limits;
use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::ActionType;
use crate::base::input::diffbelt_call::DiffbeltCallInput;
use crate::input_handler;
use crate::transform::{ActionInputHandlerAction, ActionInputHandlerActionsVec};
use diffbelt_util_no_std::either::left_if_some;
use diffbelt_util_no_std::from_either::Either;
use std::borrow::Cow;

impl AggregateTransform {
    pub fn maybe_read_cursor(
        actions: &mut ActionInputHandlerActionsVec<Self, HandlerContext>,
        max_limits: &Limits,
        current_limits: &Limits,
        from_collection_name: &str,
        stored_cursor: &mut Option<Box<str>>,
        got_cursor: Option<Box<str>>,
    ) {
        let read_cursor =
            left_if_some(got_cursor.or_else(|| stored_cursor.take())).left_and_then(|cursor| {
                if Self::can_request_diff(max_limits, current_limits) {
                    Either::Left(cursor)
                } else {
                    Either::Right(Some(cursor))
                }
            });

        match read_cursor {
            Either::Left(cursor) => {
                actions.push(Self::read_cursor(from_collection_name, &cursor));
            }
            Either::Right(cursor) => {
                *stored_cursor = cursor;
            }
        }
    }

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
