use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::request::HyperRequestWrapped;
use crate::http::routing::response::{
    BaseResponse, BytesVecResponse, Response as ResponseByRoute, StaticStrResponse, StringResponse,
};
use crate::http::routing::StaticRouteOptions;
use hyper::http::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;

use std::net::SocketAddr;
use std::sync::Arc;

async fn handle_request(
    context: Arc<Context>,
    req: Request<Body>,
) -> Result<Response<Body>, HttpError> {
    let uri = req.uri();
    let path_and_query = uri
        .path_and_query()
        .ok_or(HttpError::PublicInternal500("path_and_query"))?;

    let path = path_and_query.path();

    let routing = &context.routing;
    let routes = routing.get_static_routes_by_method(req.method().as_str());
    let static_route = routes.and_then(|routes| routes.get(path));

    let static_route = match static_route {
        None => {
            return handle_pattern_request(context, req).await;
        }
        Some(static_route) => static_route,
    };

    let request = HyperRequestWrapped::from(req);

    let result = static_route(StaticRouteOptions {
        context: context.clone(),
        request,
    })
    .await?;

    handle_response(result).await
}

async fn handle_pattern_request(
    context: Arc<Context>,
    req: Request<Body>,
) -> Result<Response<Body>, HttpError> {
    let routing = &context.routing;
    let _path = req.uri().path();

    let mut options = StaticRouteOptions {
        context: context.clone(),
        request: HyperRequestWrapped::from(req),
    };

    for route in &routing.pattern_routes {
        let handler = &route.handler;

        let result = handler(options, &route.path);

        match result {
            Ok(result) => {
                let result = result.await?;
                return handle_response(result).await;
            }
            Err(opts) => {
                options = opts;
            }
        }
    }

    return Err(HttpError::NotFound);
}

async fn handle_response(result: ResponseByRoute) -> Result<Response<Body>, HttpError> {
    let (mut response, base) = match result {
        ResponseByRoute::String(StringResponse { base, str }) => (Response::new(str.into()), base),
        ResponseByRoute::StaticStr(StaticStrResponse { base, str }) => {
            (Response::new(str.into()), base)
        }
        ResponseByRoute::BytesVec(BytesVecResponse { base, bytes }) => {
            (Response::new(bytes.into()), base)
        }
    };

    init_response(&mut response, &base)?;
    Ok(response)
}

fn init_response(response: &mut Response<Body>, base: &BaseResponse) -> Result<(), HttpError> {
    let BaseResponse {
        status,
        content_type,
    } = base;

    let status_code =
        StatusCode::from_u16(*status).or(Err(HttpError::PublicInternal500("status_code")))?;

    *response.status_mut() = status_code;

    let headers = response.headers_mut();
    headers.insert("Content-Type", content_type.parse().unwrap());

    Ok(())
}

pub async fn start_http_server(context: Arc<Context>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3030));

    let make_svc = make_service_fn(|_conn| {
        let context = context.clone();

        let fun = move |req| {
            let context = context.clone();

            async move {
                let result = handle_request(context, req).await;

                match result {
                    Ok(response) => Ok::<Response<Body>, Infallible>(response),
                    Err(err) => {
                        let mut is_json = true;

                        let (status_code, body): (StatusCode, Body) = match err {
                            HttpError::Unspecified => (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "{\"error\":\"500\"}".into(),
                            ),
                            HttpError::NotFound => {
                                (StatusCode::NOT_FOUND, "{\"error\":\"404\"}".into())
                            }
                            HttpError::Generic400(reason)
                            | HttpError::ContentTypeUnsupported(reason) => (
                                StatusCode::BAD_REQUEST,
                                format!(
                                    "{{\"error\":\"400\",\"reason\":{}}}",
                                    serde_json::json!(reason).to_string()
                                )
                                .into(),
                            ),
                            HttpError::CustomJson400(json) => (StatusCode::BAD_REQUEST, json.into()),
                            HttpError::GenericString400(reason) => {
                                is_json = false;
                                (
                                    StatusCode::BAD_REQUEST,
                                    format!(
                                        "{{\"error\":\"400\",\"reason\":{}}}",
                                        serde_json::json!(reason).to_string()
                                    )
                                    .into(),
                                )
                            }
                            HttpError::TooBigPayload(max_size) => (
                                StatusCode::PAYLOAD_TOO_LARGE,
                                format!("{{\"error\":\"413\",\"bytesMax\":{}}}", max_size).into(),
                            ),
                            HttpError::InvalidJson(reason) => (
                                StatusCode::BAD_REQUEST,
                                format!(
                                    "{{\"error\":\"400\",\"type\":\"invalidJson\",\"reason\":{}}}",
                                    serde_json::json!(reason).to_string()
                                )
                                .into(),
                            ),
                            HttpError::PublicInternal500(str) => {
                                is_json = false;
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("500, {}", str).into(),
                                )
                            }
                            HttpError::PublicInternalString500(str) => {
                                is_json = false;
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("500, {}", str).into(),
                                )
                            }
                            HttpError::MethodNotAllowed => {
                                (StatusCode::METHOD_NOT_ALLOWED, "{\"error\":\"405\"}".into())
                            }
                            err => {
                                eprintln!("Unhandled HttpError: {:?}", err);

                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "{\"error\":\"500\"}".into(),
                                )
                            }
                        };

                        let mut response = Response::new(body);
                        *(response.status_mut()) = status_code;

                        if is_json {
                            let headers = response.headers_mut();
                            headers.insert(
                                "Content-Type",
                                HeaderValue::from_static("application/json; charset=utf-8"),
                            );
                        }

                        Ok(response)
                    }
                }
            }
        };

        async { Ok::<_, Infallible>(service_fn(fun)) }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
