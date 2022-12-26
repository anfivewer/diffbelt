pub enum HttpError {
    NotFound,
    Unspecified,
    PublicInternal500(&'static str),
    PublicInternalString500(String),
    MethodNotAllowed,
}
