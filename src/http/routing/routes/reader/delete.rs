use crate::collection::methods::delete_reader::DeleteReaderOptions;
use std::sync::Arc;

use crate::collection::Collection;

use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::response::Response;

use crate::http::util::response::create_ok_no_error_json_response;

pub async fn delete_reader(
    _request: impl Request,
    collection: Arc<Collection>,
    reader_name: String,
) -> Result<Response, HttpError> {
    let options = DeleteReaderOptions { reader_name };

    let result = collection.delete_reader(options).await;

    let _ = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("reader/delete error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    create_ok_no_error_json_response()
}
