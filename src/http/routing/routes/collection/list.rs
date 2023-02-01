use crate::common::IsByteArray;
use crate::context::Context;

use futures::stream::FuturesOrdered;
use futures::StreamExt;
use std::sync::Arc;

use crate::http::routing::{StaticRouteFnResult, StaticRouteOptions};

use crate::http::util::response::create_ok_json_response;
use crate::http::validation::MethodsValidation;

use crate::collection::Collection;

use crate::util::str_serialization::StrSerializationType;
use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListCollectionsItemJsonData {
    name: String,
    is_manual: bool,
    generation_id: String,
    generation_id_encoding: Option<String>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListCollectionsResponseJsonData {
    items: Vec<ListCollectionsItemJsonData>,
}

fn handler(options: StaticRouteOptions) -> StaticRouteFnResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_methods(&["GET"])?;

        let collections = context.database.collections_list().await;

        let items: FuturesOrdered<_> = collections
            .into_iter()
            .map(|collection: Arc<Collection>| async move {
                let generation_id = collection.get_generation_id().await;

                let (generation_id, encoding) = StrSerializationType::Utf8
                    .serialize_with_priority(generation_id.get_byte_array());

                ListCollectionsItemJsonData {
                    name: collection.get_id().to_string(),
                    is_manual: collection.is_manual(),
                    generation_id,
                    generation_id_encoding: encoding.to_optional_string(),
                }
            })
            .collect();

        let items = items.collect::<Vec<ListCollectionsItemJsonData>>().await;

        create_ok_json_response(&ListCollectionsResponseJsonData { items })
    })
}

pub fn register_list_collections_route(context: &mut Context) {
    context
        .routing
        .add_static_get_route("/collection/list", handler);
}
