use crate::collection::methods::diff::ReadDiffCursorOptions;
use std::sync::Arc;

use crate::collection::Collection;

use crate::http::data::diff_response::DiffResponseJsonData;

use crate::http::errors::HttpError;
use crate::http::request::Request;

use crate::http::routing::response::Response;

use crate::http::util::response::create_ok_json_response;

pub async fn read_cursor(
    _request: impl Request,
    collection: Arc<Collection>,
    cursor_id: String,
) -> Result<Response, HttpError> {
    let options = ReadDiffCursorOptions { cursor_id };

    let result = collection.read_diff_cursor(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("diff/next error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let response = DiffResponseJsonData::from(result);
    create_ok_json_response(&response)
}
