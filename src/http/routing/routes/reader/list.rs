use crate::collection::methods::list_readers::ListReadersOk;
use crate::collection::Collection;
use crate::http::data::reader_record::ReaderRecordJsonData;
use crate::http::errors::HttpError;
use crate::http::request::Request;
use crate::http::routing::response::Response;
use crate::http::util::response::create_ok_json_response;
use std::sync::Arc;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseJsonData {
    items: Vec<ReaderRecordJsonData>,
}

pub async fn list_readers(
    _request: impl Request,
    collection: Arc<Collection>,
) -> Result<Response, HttpError> {
    let result = collection.list_readers().await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            eprintln!("reader/list error {:?}", err);
            return Err(HttpError::Unspecified);
        }
    };

    let ListReadersOk { items } = result;

    let response = ResponseJsonData {
        items: ReaderRecordJsonData::encode_vec(items),
    };

    create_ok_json_response(&response)
}
