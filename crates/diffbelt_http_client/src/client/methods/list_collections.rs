use hyper::{Body, Method, Request};
use diffbelt_types::collection::list::ListCollectionsResponseJsonData;
use diffbelt_util::http::read_full_body::into_full_body_as_read;
use crate::client::DiffbeltClient;
use crate::constants::MAX_LIST_COLLECTIONS_RESPONSE_BYTES;
use crate::errors::DiffbeltClientError;

impl DiffbeltClient {
    pub async fn list_collections(
        &self,
    ) -> Result<ListCollectionsResponseJsonData, DiffbeltClientError> {
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("{}/collections/", self.uri_start))
            .body(Body::empty())
            .unwrap();

        let res = self.client.request(req).await?;

        let body = res.into_body();

        let body = into_full_body_as_read(body, MAX_LIST_COLLECTIONS_RESPONSE_BYTES).await?;

        let response: ListCollectionsResponseJsonData =
            serde_json::from_reader(body).map_err(|_| DiffbeltClientError::JsonParsing)?;

        Ok(response)
    }
}
