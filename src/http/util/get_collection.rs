use crate::collection::Collection;
use crate::context::Context;
use crate::http::errors::HttpError;
use std::sync::Arc;

pub async fn get_collection(
    context: &Context,
    collection_id: &str,
) -> Result<Arc<Collection>, HttpError> {
    let collection = context.database.get_collection(&collection_id).await;
    let Some(collection) = collection else { return Err(HttpError::Generic400("no such collection")); };

    Ok(collection)
}
