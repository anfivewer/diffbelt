use crate::http::errors::HttpError;
use crate::http::routing::response::{
    BaseResponse, BytesVecResponse, FlatbuffersResponse, HttpResponse, StaticStrResponse,
};
use diffbelt_protos::protos::impls::ResponseProto;
use diffbelt_protos::OwnedSerialized;
use serde::Serialize;

pub fn create_ok_static_str_json_response<E>(str: &'static str) -> Result<HttpResponse, E> {
    Ok(HttpResponse::StaticStr(StaticStrResponse {
        base: BaseResponse {
            content_type: "application/json; charset=utf-8",
            ..Default::default()
        },
        str,
    }))
}

pub fn create_ok_no_error_json_response<E>() -> Result<HttpResponse, E> {
    create_ok_static_str_json_response(r#"{"error":null}"#)
}

pub fn create_ok_json_response<T: Serialize>(response: &T) -> Result<HttpResponse, HttpError> {
    let response = serde_json::to_vec(&response).or(Err(HttpError::PublicInternal500(
        "result serialization failed",
    )))?;

    Ok(HttpResponse::BytesVec(BytesVecResponse {
        base: BaseResponse {
            content_type: "application/json; charset=utf-8",
            ..Default::default()
        },
        bytes: response,
    }))
}

pub fn create_ok_flatbuffers_response<'a>(
    serialized: OwnedSerialized<ResponseProto>,
) -> Result<HttpResponse, HttpError> {
    Ok(HttpResponse::Flatbuffers(FlatbuffersResponse {
        base: BaseResponse {
            content_type: "application/x-flatbuffers",
            ..Default::default()
        },
        bytes: serialized.into_aligned_bytes(),
    }))
}
