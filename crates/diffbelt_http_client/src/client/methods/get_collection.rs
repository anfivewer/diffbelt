use crate::client::DiffbeltClient;
use crate::constants::MAX_GET_COLLECTION_RESPONSE_BYTES;
use crate::errors::DiffbeltClientError;
use diffbelt_types::collection::get::GetCollectionResponseJsonData;
use diffbelt_util::http::read_full_body::into_full_body_as_read;
use hyper::{Body, Method, Request};

impl DiffbeltClient {
    pub async fn get_collection(
        &self,
        collection_name: &str,
    ) -> Result<GetCollectionResponseJsonData, DiffbeltClientError> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "{}/collections/{}",
                self.uri_start, collection_name
            ))
            .body(Body::empty())
            .unwrap();

        let res = self.client.request(req).await?;

        let status = res.status();

        let body = res.into_body();

        let body = into_full_body_as_read(body, MAX_GET_COLLECTION_RESPONSE_BYTES).await?;

        let response: GetCollectionResponseJsonData =
            serde_json::from_reader(body).map_err(|_| DiffbeltClientError::JsonParsing)?;

        Ok(response)
    }
}
