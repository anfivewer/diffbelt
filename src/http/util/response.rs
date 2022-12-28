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
