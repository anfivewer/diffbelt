use diffbelt_types::collection::diff::DiffCollectionRequestJsonData;
use diffbelt_types::collection::generation::{
    CommitGenerationRequestJsonData, StartGenerationRequestJsonData,
};
use diffbelt_types::collection::put_many::PutManyRequestJsonData;
use std::borrow::Cow;

#[derive(Debug, Eq, PartialEq)]
pub enum Method {
    Get,
    Post,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DiffbeltRequestBody {
    ReadDiffCursorNone,
    DiffCollectionStart(DiffCollectionRequestJsonData),
    StartGeneration(StartGenerationRequestJsonData),
    CommitGeneration(CommitGenerationRequestJsonData),
    PutMany(PutManyRequestJsonData),
}

#[derive(Debug, Eq, PartialEq)]
pub struct DiffbeltCallAction {
    pub method: Method,
    pub path: Cow<'static, str>,
    pub query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    pub body: DiffbeltRequestBody,
}
