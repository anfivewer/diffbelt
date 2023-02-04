use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;

use crate::http::errors::HttpError;
use crate::http::routing::response::Response;

use crate::http::util::response::create_ok_no_error_json_response;

use std::sync::Arc;

pub async fn delete_collection(collection: Arc<Collection>) -> Result<Response, HttpError> {
    let result = collection.delete_collection();

    drop(collection);

    let result = result.await;

    if let Err(err) = result {
        return match err {
            CollectionMethodError::NoSuchCollection => create_ok_no_error_json_response(),
            _ => {
                eprintln!("delete collection error {:?}", err);
                Err(HttpError::Unspecified)
            }
        };
    }

    create_ok_no_error_json_response()
}
