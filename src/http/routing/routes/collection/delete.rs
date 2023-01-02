use crate::collection::methods::errors::CollectionMethodError;
use crate::context::Context;
use crate::http::constants::DELETE_COLLECTION_REQUEST_MAX_BYTES;
use crate::http::errors::HttpError;
use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_no_error_json_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteCollectionRequestJsonData {
    collection_id: String,
}

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_methods(&["POST"])?;
        request.allow_only_utf8_json_by_default()?;

        let body = read_limited_body(request, DELETE_COLLECTION_REQUEST_MAX_BYTES).await?;
        let data: DeleteCollectionRequestJsonData = read_json(body)?;

        let collection_id = data.collection_id;

        let result = context.database.get_collection(&collection_id).await;

        let Some(collection) = result else {
            return Err(HttpError::Generic400("no such collection"));
        };

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
    })
}

pub fn register_delete_collection_route(context: &mut Context) {
    context
        .routing
        .add_static_get_route("/collection/delete", handler);
}
