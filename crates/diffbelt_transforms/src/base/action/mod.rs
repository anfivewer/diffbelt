use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::function_eval::FunctionEvalAction;
use diffbelt_types::collection::diff::{DiffCollectionRequestJsonData, ReaderDiffFromDefJsonData};
use std::borrow::Cow;

pub mod diffbelt_call;
pub mod function_eval;

#[derive(Debug)]
pub struct Action {
    pub id: (u64, u64),
    pub action: ActionType,
}

#[derive(Debug)]
pub enum ActionType {
    DiffbeltCall(DiffbeltCallAction),
    FunctionEval(FunctionEvalAction),
}

impl ActionType {
    pub fn new_diff_call_by_reader(
        collection_name: &str,
        reader_name: &str,
        reader_collection_name: &str,
    ) -> Self {
        Self::DiffbeltCall(DiffbeltCallAction {
            method: Method::Post,
            path: Cow::Owned(format!(
                "/collections/{}/diff/",
                urlencoding::encode(collection_name)
            )),
            query: Vec::with_capacity(0),
            body: DiffbeltRequestBody::DiffCollectionStart(DiffCollectionRequestJsonData {
                from_generation_id: None,
                to_generation_id: None,
                from_reader: Some(ReaderDiffFromDefJsonData {
                    reader_name: reader_name.to_string(),
                    collection_name: Some(reader_collection_name.to_string()),
                }),
            }),
        })
    }
}

#[cfg(test)]
impl ActionType {
    pub fn as_diffbelt_call(&self) -> Option<&DiffbeltCallAction> {
        let ActionType::DiffbeltCall(call) = self else {
            return None;
        };

        Some(call)
    }
}
