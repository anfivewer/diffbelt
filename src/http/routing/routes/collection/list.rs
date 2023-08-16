use crate::context::Context;
use std::sync::Arc;

use futures::stream::FuturesOrdered;
use futures::StreamExt;

use crate::http::routing::{StaticRouteFnFutureResult, StaticRouteOptions};

use crate::http::util::response::create_ok_json_response;
use crate::http::validation::MethodsValidation;

use crate::collection::Collection;

use diffbelt_types::collection::list::{
    ListCollectionsItemJsonData, ListCollectionsResponseJsonData,
};

fn handler(options: StaticRouteOptions) -> StaticRouteFnFutureResult {
    Box::pin(async move {
        let context = options.context;
        let request = options.request;

        request.allow_only_methods(&["GET"])?;

        let collections = context.database.collections_list().await;

        let items: FuturesOrdered<_> = collections
            .into_iter()
            .map(|collection: Arc<Collection>| async move {
                ListCollectionsItemJsonData {
                    name: collection.get_name().to_string(),
                    is_manual: collection.is_manual(),
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
        .add_static_get_route("/collections/", handler);
}
