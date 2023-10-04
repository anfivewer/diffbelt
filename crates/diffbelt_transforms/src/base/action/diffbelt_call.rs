use diffbelt_types::collection::diff::DiffCollectionRequestJsonData;
use diffbelt_types::collection::generation::StartGenerationRequestJsonData;
use std::borrow::Cow;

pub enum Method {
    Get,
    Post,
}

pub enum DiffbeltRequestBody {
    None,
    DiffCollectionStart(DiffCollectionRequestJsonData),
    StartGeneration(StartGenerationRequestJsonData),
}

pub struct DiffbeltCallAction {
    pub method: Method,
    pub path: Cow<'static, str>,
    pub query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    pub body: DiffbeltRequestBody,
}
