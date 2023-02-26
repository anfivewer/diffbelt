#[derive(Debug)]
pub enum HttpError {
    NotFound,
    Unspecified,
    Generic400(&'static str),
    GenericString400(String),
    CustomJson400(&'static str),
    /** max_size */
    TooBigPayload(usize),
    InvalidJson(String),
    PublicInternal500(&'static str),
    MethodNotAllowed,
    ContentTypeUnsupported(&'static str),
}
