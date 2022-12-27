#[derive(Debug)]
pub enum HttpError {
    NotFound,
    Unspecified,
    Generic400(&'static str),
    GenericString400(String),
    /** max_size */
    TooBigPayload(usize),
    InvalidJson,
    PublicInternal500(&'static str),
    PublicInternalString500(String),
    MethodNotAllowed,
    ContentTypeUnsupported(&'static str),
}
