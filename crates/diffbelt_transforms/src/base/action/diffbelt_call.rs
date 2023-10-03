use diffbelt_types::collection::get::GetCollectionRequestJsonData;
use std::borrow::Cow;

pub enum Method {
    Post,
}

pub enum DiffbeltRequestBody {
    GetCollection(GetCollectionRequestJsonData),
}

pub struct DiffbeltCallAction {
    pub method: Method,
    pub path: Cow<'static, str>,
    pub body: DiffbeltRequestBody,
}
