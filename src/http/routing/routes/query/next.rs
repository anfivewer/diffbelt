use std::sync::Arc;

use crate::collection::Collection;

use crate::collection::methods::query::ReadQueryCursorOptions;

use crate::http::data::query_response::QueryResponseJsonData;
use crate::http::errors::HttpError;
use crate::http::request::Request;

use crate::http::routing::response::Response;

use crate::http::util::response::create_ok_json_response;

pub async fn read_cursor(
    _request: impl Request,
    collection: Arc<Collection>,
    cursor_id: String,
) -> Result<Response, HttpError> {
    let options = ReadQueryCursorOptions { cursor_id };

    let result = collection.read_query_cursor(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("query/start error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = QueryResponseJsonData::from(result);
    create_ok_json_response(&response)
}
