use crate::context::Context;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::request::HyperRequestWrapped;
use crate::http::routing::response::{
    BaseResponse, BytesVecResponse, FlatbuffersResponse, HttpResponse as ResponseByRoute,
    StaticStrResponse, StringResponse,
};
use crate::http::routing::StaticRouteOptions;
use diffbelt_aligned_bytes::AlignedBytes;
use diffbelt_protos::protos::api::common::{ErrorResponse, ErrorResponseArgs};
use diffbelt_protos::protos::api::methods::{ResponseArgs, ResponseBody};
use diffbelt_protos::protos::impls::ResponseProto;
use diffbelt_protos::{FlatbuffersGenericType, Serialized, Serializer};
use diffbelt_util::idling_status::BusyTask;
use diffbelt_util_no_std::on_drop::OnDrop;
use hyper::body::Bytes;
use hyper::http::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{span, trace, trace_span, Level, Span};

async fn handle_request(
    context: Arc<Context>,
    request_context: RequestContext,
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
            return handle_pattern_request(context, request_context, req).await;
        }
        Some(static_route) => static_route,
    };

    let request = HyperRequestWrapped::from(req);

    let result = static_route(StaticRouteOptions {
        context: context.clone(),
        request_context,
        request,
    })
    .await?;

    handle_response(result).await
}

async fn handle_pattern_request(
    context: Arc<Context>,
    request_context: RequestContext,
    req: Request<Body>,
) -> Result<Response<Body>, HttpError> {
    let routing = &context.routing;
    let _path = req.uri().path();

    let mut options = StaticRouteOptions {
        context: context.clone(),
        request_context,
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

    Err(HttpError::NotFound)
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
        ResponseByRoute::Flatbuffers(FlatbuffersResponse { base, bytes }) => {
            // TODO: streaming?
            (
                Response::new(Bytes::copy_from_slice(bytes.as_slice()).into()),
                base,
            )
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

pub async fn start_http_server(context: Arc<Context>, task: BusyTask) {
    const PORT: u16 = 3030;

    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));

    let request_id_counter = Box::leak(Box::new(AtomicU64::new(1))) as &AtomicU64;

    let make_svc = make_service_fn(|_conn| {
        let context = context.clone();

        let fun = move |req: Request<Body>| {
            let span = trace_span!(
                "request",
                id = request_id_counter.fetch_add(1, Ordering::Relaxed),
            );
            trace!(
                parent: &span,
                "start {}",
                req.uri()
                    .path_and_query()
                    .map(|x| x.as_str())
                    .unwrap_or("?")
            );

            struct ResultStatus {
                span: Span,
                status: StatusCode,
            }

            let mut result_trace = OnDrop::new(
                |params| {
                    trace!(parent: &params.span, "end status:{}", params.status.as_u16());
                },
                ResultStatus {
                    span: span.clone(),
                    status: StatusCode::IM_A_TEAPOT,
                },
            );

            let context = context.clone();

            let task = context.idling.start_work();

            async move {
                let is_flatbuffers_atomic = Arc::new(AtomicBool::new(false));
                let request_context = RequestContext {
                    is_flatbuffers_: is_flatbuffers_atomic.clone(),
                    span: span.clone(),
                };
                let result = handle_request(context, request_context, req).await;

                match result {
                    Ok(response) => {
                        result_trace.params_mut().status = response.status();

                        Ok::<Response<Body>, Infallible>(response)
                    }
                    Err(err) => {
                        let mut make_flatbuffers_error =
                            |error_str: Option<&str>, reason: Option<&str>, details: Option<&str>| {
                                let mut serializer = Serializer::<ResponseProto>::new();
                                let error_str = if let Some(error_str) = error_str {
                                    Some(serializer.create_string(error_str))
                                } else {
                                    None
                                };
                                let reason = if let Some(reason) = reason {
                                    Some(serializer.create_string(reason))
                                } else {
                                    None
                                };
                                let details = if let Some(details) = details {
                                    Some(serializer.create_string(&details))
                                } else {
                                    None
                                };
                                let error = ErrorResponse::create(
                                    serializer.buffer_builder(),
                                    &ErrorResponseArgs {
                                        code: 400,
                                        error: error_str,
                                        reason,
                                        details,
                                    },
                                );
                                let response =
                                    <ResponseProto as FlatbuffersGenericType>::FlatType::create(
                                        serializer.buffer_builder(),
                                        &ResponseArgs {
                                            body_type: ResponseBody::Error,
                                            body: Some(error.as_union_value()),
                                        },
                                    );
                                let serialized = serializer.finish(response);
                                let serialized = serialized.as_bytes().to_vec();

                                serialized.into()
                            };

                        trace!(parent: &span, "error: {err:?}");

                        let is_flatbuffers = is_flatbuffers_atomic.load(Ordering::Relaxed);

                        let (status_code, body) =
                            map_http_err(err, is_flatbuffers, make_flatbuffers_error);

                        let mut response = Response::new(body);
                        *(response.status_mut()) = status_code;
                        result_trace.params_mut().status = status_code;

                        if is_flatbuffers {
                            let headers = response.headers_mut();
                            headers.insert(
                                "Content-Type",
                                HeaderValue::from_static("application/x-flatbuffers"),
                            );
                        } else {
                            let headers = response.headers_mut();
                            headers.insert(
                                "Content-Type",
                                HeaderValue::from_static("application/json; charset=utf-8"),
                            );
                        }

                        drop(task);

                        Ok(response)
                    }
                }
            }
        };

        async { Ok::<_, Infallible>(service_fn(fun)) }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("Started at port {PORT}");

    drop(task);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn map_http_err<F: FnOnce(Option<&str>, Option<&str>, Option<&str>) -> Body>(
    err: HttpError,
    is_flatbuffers: bool,
    make_flatbuffers_error: F,
) -> (StatusCode, Body) {
    match err {
        HttpError::Unspecified => (
            StatusCode::INTERNAL_SERVER_ERROR,
            if is_flatbuffers {
                make_flatbuffers_error(None, Some("500"), None)
            } else {
                "{\"error\":\"500\"}".into()
            },
        ),
        HttpError::NotFound => (
            StatusCode::NOT_FOUND,
            if is_flatbuffers {
                make_flatbuffers_error(None, Some("notFound"), None)
            } else {
                "{\"error\":\"404\"}".into()
            },
        ),
        HttpError::Generic400(reason) | HttpError::ContentTypeUnsupported(reason) => (
            StatusCode::BAD_REQUEST,
            if is_flatbuffers {
                make_flatbuffers_error(None, None, Some(reason))
            } else {
                format!(
                    "{{\"error\":\"400\",\"details\":{}}}",
                    serde_json::json!(reason).to_string()
                )
                .into()
            },
        ),
        HttpError::GenericString400(reason) => (
            StatusCode::BAD_REQUEST,
            if is_flatbuffers {
                make_flatbuffers_error(None, None, Some(&reason))
            } else {
                format!(
                    "{{\"error\":\"400\",\"details\":{}}}",
                    serde_json::json!(reason).to_string()
                )
                .into()
            },
        ),
        HttpError::InvalidFlatbuffers(details) => (
            StatusCode::BAD_REQUEST,
            make_flatbuffers_error(None, Some("invalidFlatbuffers"), Some(&details)),
        ),
        HttpError::GenericFlatbuffers400(details) => (
            StatusCode::BAD_REQUEST,
            make_flatbuffers_error(None, None, Some(details)),
        ),
        HttpError::TooBigPayload(max_size) => (
            StatusCode::PAYLOAD_TOO_LARGE,
            if is_flatbuffers {
                make_flatbuffers_error(
                    None,
                    Some("payloadTooLarge"),
                    Some(&format!("bytesMax:{max_size}")),
                )
            } else {
                format!("{{\"error\":\"413\",\"bytesMax\":{}}}", max_size).into()
            },
        ),
        HttpError::InvalidJson(reason) => (
            StatusCode::BAD_REQUEST,
            format!(
                "{{\"error\":\"400\",\"reason\":\"invalidJson\",\"details\":{}}}",
                serde_json::json!(reason).to_string()
            )
            .into(),
        ),
        HttpError::PublicInternal500(str) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            if is_flatbuffers {
                make_flatbuffers_error(None, Some("500"), Some(str))
            } else {
                format!(
                    r#"{{"error":"500","details":{}}}"#,
                    serde_json::json!(str).to_string()
                )
                .into()
            },
        ),
        HttpError::MethodNotAllowed => {
            (StatusCode::METHOD_NOT_ALLOWED, "{\"error\":\"405\"}".into())
        }
        HttpError::NoSuchCollection => (
            StatusCode::BAD_REQUEST,
            if is_flatbuffers {
                make_flatbuffers_error(Some("noSuchCollection"), None, None)
            } else {
                r#"{"error":"404","reason":"noSuchCollection"}"#.into()
            },
        ),
        HttpError::Custom {
            status_code,
            error,
            reason,
            details,
        } => (
            status_code,
            if is_flatbuffers {
                make_flatbuffers_error(error, reason, details)
            } else {
                format!(
                    r#"{{"error":{},"reason":{},"details":{}}}"#,
                    error
                        .map(|x| serde_json::json!(error).to_string())
                        .unwrap_or(String::from("null")),
                    reason
                        .map(|x| serde_json::json!(error).to_string())
                        .unwrap_or(String::from("null")),
                    details
                        .map(|x| serde_json::json!(error).to_string())
                        .unwrap_or(String::from("null")),
                )
                .into()
            },
        ),
    }
}
