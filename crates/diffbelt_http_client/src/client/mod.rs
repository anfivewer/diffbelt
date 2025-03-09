use crate::constants::MAX_RESPONSE_BYTES;
use crate::errors::DiffbeltClientError;
use crate::util::body::{ExpectedResponseType, TransformBodyTrait};
use crate::util::http::TransformMethodTrait;
use diffbelt_protos::protos::api::common::ErrorResponse;
use diffbelt_protos::protos::handlers::ApiHandler;
use diffbelt_protos::protos::impls::{RequestProto, ResponseProto};
use diffbelt_protos::{deserialize, FlatbuffersGenericType, OwnedSerialized};
use diffbelt_transforms::base::action::diffbelt_call::DiffbeltCallAction;
use diffbelt_transforms::base::input::diffbelt_call::DiffbeltResponseBody;
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::collection::get_record::GetResponseJsonData;
use diffbelt_types::collection::put_many::PutManyResponseJsonData;
use diffbelt_util::http::read_full_body::into_full_body_as_read;
use diffbelt_util::http::read_to_aligned_bytes::into_aligned_bytes;
use hyper::body::Bytes;
use hyper::client::HttpConnector;
use hyper::{Body, Client, Request};
use std::fmt::Write;
use std::future::Future;
use std::io::Read;
use std::marker::PhantomData;

pub mod methods;

pub struct DiffbeltClientNewOptions {
    pub host: String,
    pub port: u16,
}

pub struct AnotherClient {}

pub struct DiffbeltClient {
    uri_start: String,
    client: Client<HttpConnector, Body>,
}

impl DiffbeltClient {
    pub fn new(options: DiffbeltClientNewOptions) -> Self {
        let DiffbeltClientNewOptions { host, port } = options;

        Self {
            uri_start: format!("http://{}:{}", host, port),
            client: Client::builder().build_http(),
        }
    }

    pub async fn transform_call(
        &self,
        action: DiffbeltCallAction,
    ) -> Result<DiffbeltResponseBody, DiffbeltClientError> {
        let DiffbeltCallAction {
            method,
            path,
            query,
            body,
        } = action;

        if !query.is_empty() {
            todo!("Query params is not yet supported");
        }

        let (body, expected_response_type) = body
            .into_hyper_body()
            .map_err(DiffbeltClientError::JsonSerialize)?;

        let req = Request::builder()
            .method(method.into_hyper_method())
            .uri(format!("{}{}", self.uri_start, path))
            .body(body)
            .unwrap();

        let res = self.client.request(req).await?;

        let status = res.status();

        let body = res.into_body();

        let mut body = into_full_body_as_read(body, MAX_RESPONSE_BYTES).await?;

        if status != 200 {
            let mut s = String::new();
            let _: usize = body
                .read_to_string(&mut s)
                .map_err(|_| DiffbeltClientError::Not200Unknown)?;

            return Err(DiffbeltClientError::Not200(s));
        }

        match expected_response_type {
            ExpectedResponseType::Ok => Ok(DiffbeltResponseBody::Ok(())),
            ExpectedResponseType::Diff => {
                let response: DiffCollectionResponseJsonData =
                    serde_json::from_reader(body).map_err(|_| DiffbeltClientError::JsonParsing)?;

                Ok(DiffbeltResponseBody::Diff(response))
            }
            ExpectedResponseType::PutMany => {
                let response: PutManyResponseJsonData =
                    serde_json::from_reader(body).map_err(|_| DiffbeltClientError::JsonParsing)?;

                Ok(DiffbeltResponseBody::PutMany(response))
            }
            ExpectedResponseType::GetRecord => {
                let response: GetResponseJsonData =
                    serde_json::from_reader(body).map_err(|_| DiffbeltClientError::JsonParsing)?;

                Ok(DiffbeltResponseBody::GetRecord(response))
            }
        }
    }

    pub async fn flatbuffers_raw_call(
        &self,
        request: OwnedSerialized<RequestProto>,
    ) -> Result<OwnedSerialized<ResponseProto>, DiffbeltClientError> {
        let body = Body::from(Bytes::copy_from_slice(request.as_bytes()));

        let req = Request::builder()
            .method("POST")
            .uri(format!("{}/flatbuffers", self.uri_start))
            .body(body)
            .unwrap();

        let res = self.client.request(req).await?;

        let status = res.status();

        let body = res.into_body();
        let body = into_aligned_bytes(body, MAX_RESPONSE_BYTES).await?;

        let response = OwnedSerialized::<ResponseProto>::from_aligned_bytes(body)?;

        if status != 200 {
            let error = response.data().body_as_error();
            let Some(error) = error else {
                return Err(DiffbeltClientError::Not200Unknown);
            };

            let code = error.code();
            let reason = error.reason();
            let details = error.details();

            let mut s = String::with_capacity(
                "Code 666, reason: , details: ".len()
                    + reason.map_or(2, |x| x.len())
                    + details.map_or(2, |x| x.len()),
            );

            if let Err(_) = s.write_fmt(format_args!(
                "Code: {}, reason: {}, details: {}",
                code,
                reason.unwrap_or("()"),
                details.unwrap_or("()"),
            )) {
                s.push_str("Format error");
            };

            return Err(DiffbeltClientError::Not200(s));
        }

        Ok(response)
    }

    pub async fn flatbuffers_call<A: ApiHandler>(
        &self,
        request: OwnedSerialized<RequestProto>,
    ) -> Result<FlatbuffersResponse<A>, DiffbeltClientError> {
        let response = self.flatbuffers_raw_call(request).await?;

        Ok(FlatbuffersResponse {
            response,
            phantom: Default::default(),
        })
    }
}

pub struct FlatbuffersResponse<A: ApiHandler> {
    response: OwnedSerialized<ResponseProto>,
    phantom: PhantomData<A>,
}

impl<A: ApiHandler> FlatbuffersResponse<A> {
    pub fn response(
        &self,
    ) -> Result<
        <A::FlatbuffersResponse as FlatbuffersGenericType>::FlatType<'_>,
        Option<ErrorResponse<'_>>,
    > {
        let data = self.response.data();
        if let Some(error) = data.body_as_error() {
            return Err(Some(error));
        }

        if let Some(ok) = A::response(data) {
            return Ok(ok);
        }

        Err(None)
    }
}
