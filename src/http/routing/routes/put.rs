use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::routing::response::{Response, StringResponse};
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};

use crate::http::request::{Request, RequestReadError};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PutRequestJsonData {
    key: String,
    #[serde(default)]
    key_encoding: Option<String>,
    value: String,
    #[serde(default)]
    value_encoding: Option<String>,
}

const MAX_SIZE: usize = 4 * 1024 * 1024;

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        let request = options.request;

        request.allow_only_methods(&["POST"])?;
        request.allow_only_utf8_json_by_default()?;

        let body = request
            .into_full_body_as_read(MAX_SIZE)
            .await
            .map_err(|err| match err {
                RequestReadError::IO => HttpError::Generic400("io"),
                RequestReadError::SizeLimit => HttpError::TooBigPayload(MAX_SIZE),
            })?;

        let data: PutRequestJsonData =
            serde_json::from_reader(body).or(Err(HttpError::InvalidJson))?;

        Ok(Response::String(StringResponse {
            base: Default::default(),
            str: format!("This is put, data: {:?}", data),
        }))
    })
}

pub fn register_put_route(context: &mut Context) {
    context.routing.add_static_get_route("/put", handler);
}
