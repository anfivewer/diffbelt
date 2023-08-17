use crate::http::errors::HttpError;
use crate::http::routing::response::{BaseResponse, BytesVecResponse, Response, StaticStrResponse};
use serde::Serialize;

pub fn create_ok_static_str_json_response<E>(str: &'static str) -> Result<Response, E> {
    Ok(Response::StaticStr(StaticStrResponse {
        base: BaseResponse {
            content_type: "application/json; charset=utf-8",
            ..Default::default()
        },
        str,
    }))
}

pub fn create_ok_no_error_json_response<E>() -> Result<Response, E> {
    create_ok_static_str_json_response(r#"{"error":null}"#)
}

pub fn create_ok_json_response<T: Serialize>(response: &T) -> Result<Response, HttpError> {
    let response = serde_json::to_vec(&response).or(Err(HttpError::PublicInternal500(
        "result serialization failed",
    )))?;

    Ok(Response::BytesVec(BytesVecResponse {
        base: BaseResponse {
            content_type: "application/json; charset=utf-8",
            ..Default::default()
        },
        bytes: response,
    }))
}
