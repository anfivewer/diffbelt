use crate::http::routing::response::{BaseResponse, Response, StaticStrResponse};

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
