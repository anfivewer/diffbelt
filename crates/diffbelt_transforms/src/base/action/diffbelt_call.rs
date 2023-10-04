use std::borrow::Cow;

pub enum Method {
    Post,
}

pub enum DiffbeltRequestBody {
    None,
}

pub struct DiffbeltCallAction {
    pub method: Method,
    pub path: Cow<'static, str>,
    pub query: Vec<(Cow<'static, str>, Cow<'static, str>)>,
    pub body: DiffbeltRequestBody,
}
