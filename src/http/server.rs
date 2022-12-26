use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::request::HyperRequest;
use crate::http::routing::response::{BaseResponse, Response as ResponseByRoute, StringResponse};
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
    let static_route = routing.static_get_routes.get(path);

    let static_route = match static_route {
        None => {
            return Err(HttpError::NotFound);
        }
        Some(static_route) => static_route,
    };

    let request = HyperRequest::from(req);

    let result = static_route(StaticRouteOptions {
        context: context.clone(),
        request,
    })
    .await?;

    match result {
        ResponseByRoute::String(StringResponse {
            base: BaseResponse { status },
            str,
        }) => {
            let mut response = Response::new(str.into());

            let status_code = StatusCode::from_u16(status)
                .or(Err(HttpError::PublicInternal500("status_code")))?;

            *response.status_mut() = status_code;

            Ok(response)
        }
    }
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
                            HttpError::NotFound => {
                                (StatusCode::NOT_FOUND, "{\"error\":\"404\"}".into())
                            }
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
                            _ => (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "{\"error\":\"500\"}".into(),
                            ),
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
