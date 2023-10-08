use crate::constants::MAX_RESPONSE_BYTES;
use crate::errors::DiffbeltClientError;
use crate::util::body::{ExpectedResponseType, TransformBodyTrait};
use crate::util::http::TransformMethodTrait;
use diffbelt_transforms::base::action::diffbelt_call::DiffbeltCallAction;
use diffbelt_transforms::base::input::diffbelt_call::DiffbeltResponseBody;
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::collection::put_many::PutManyResponseJsonData;
use diffbelt_util::http::read_full_body::into_full_body_as_read;
use hyper::client::HttpConnector;
use hyper::{Body, Client, Request};
use std::io::Read;

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
        }
    }
}
