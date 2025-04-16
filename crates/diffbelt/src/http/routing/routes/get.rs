use crate::collection::methods::get::{CollectionGetOk, CollectionGetOptions};
use crate::common::{
    IsByteArray, KeyValue as CommonKeyValue, OwnedCollectionKey, OwnedGenerationId, OwnedPhantomId,
};
use crate::context::Context;
use crate::http::constants::GET_REQUEST_MAX_BYTES;
use crate::http::data::bytes_generation_id::{
    flatbuffers_generation_id_to_owned_generation_id_opt,
    flatbuffers_phantom_id_to_owned_phantom_id_opt,
};
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_decode_opt, encoded_generation_id_data_encode,
};
use crate::http::data::encoded_key::{EncodedKeyJsonData, EncodedKeyJsonDataTrait};
use crate::http::data::encoded_phantom_id::EncodedPhantomIdJsonDataTrait;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::query::serialize_query_ok;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::{create_ok_flatbuffers_response, create_ok_json_response};
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;
use diffbelt_macro::fn_box_pin_async;
use diffbelt_protos::protos::api::common::{KeyValue, KeyValueArgs};
use diffbelt_protos::protos::api::get::GetResponseArgs;
use diffbelt_protos::protos::handlers::{
    ApiHandler, ApiHandlerRequestFlatType, GetApiHandler, StartQueryApiHandler,
};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::get_record::{GetRequestJsonData, GetResponseJsonData};
use diffbelt_types::common::phantom_id::EncodedPhantomIdJsonData;
use regex::Regex;
use std::sync::Arc;
use tracing::info;

struct UnifiedRequestData<'a> {
    collection_name: &'a str,
    key: OwnedCollectionKey,
    generation_id: Option<OwnedGenerationId>,
    phantom_id: Option<OwnedPhantomId>,
}

async fn unified_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    data: UnifiedRequestData<'_>,
) -> HttpHandlerResult {
    let collection = context.database.get_collection(data.collection_name).await;
    let Some(collection) = collection else {
        return Err(HttpError::Generic400("no such collection"));
    };

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let options = CollectionGetOptions {
        key: data.key,
        generation_id: data.generation_id,
        phantom_id: data.phantom_id,
    };

    let result = collection.get(options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            info!(parent: &request_context.span, "error {err:?}");
            return Err(HttpError::Unspecified);
        }
    };

    if request_context.is_flatbuffers() {
        let mut serializer = Serializer::new();
        let CollectionGetOk {
            generation_id,
            item,
        } = result;
        let generation_id = Some(serializer.create_vector(generation_id.get_byte_array()));
        let item = item.map(|kv| {
            let CommonKeyValue { key, value } = kv;
            let key = Some(serializer.create_vector(key.get_byte_array()));
            let value = Some(serializer.create_vector(value.get_value()));
            KeyValue::create(serializer.buffer_builder(), &KeyValueArgs { key, value })
        });
        let response = GetResponseArgs {
            generation_id,
            item,
        };
        let response = GetApiHandler::create_response(serializer, response);
        create_ok_flatbuffers_response(response)
    } else {
        let response = GetResponseJsonData {
            generation_id: encoded_generation_id_data_encode(
                result.generation_id.as_ref(),
                StrSerializationType::Utf8,
            ),
            item: result.item.map(|item| item.into()),
        };

        create_ok_json_response(&response)
    }
}

#[fn_box_pin_async]
async fn json_handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, GET_REQUEST_MAX_BYTES).await?;
    let data: GetRequestJsonData = read_json(body)?;

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let key = EncodedKeyJsonData::decode(data.key, &decoder)?;
    let generation_id = encoded_generation_id_data_decode_opt(data.generation_id)?;
    let phantom_id = EncodedPhantomIdJsonData::decode_opt(data.phantom_id, &decoder)?;

    unified_handler(
        context,
        options.request_context,
        UnifiedRequestData {
            collection_name: &collection_name,
            key,
            generation_id,
            phantom_id,
        },
    )
    .await
}

pub fn register_get_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/get$").unwrap(),
        id_only_group,
        json_handler,
    );
}

pub async fn get_flatbuffers_route(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'_, GetApiHandler>,
) -> HttpHandlerResult {
    let collection_name = request
        .collection_name()
        .ok_or_else(|| HttpError::Generic400("no collection name"))?;
    let key = request
        .key()
        .ok_or_else(|| HttpError::Generic400("no key"))?
        .bytes();
    let key = OwnedCollectionKey::from_boxed_slice(Box::from(key))
        .map_err(|_| HttpError::Generic400("invalid key"))?;
    let generation_id =
        flatbuffers_generation_id_to_owned_generation_id_opt(request.generation_id())?;
    let phantom_id = flatbuffers_phantom_id_to_owned_phantom_id_opt(request.phantom_id())?;

    let data = UnifiedRequestData {
        collection_name,
        key,
        generation_id,
        phantom_id,
    };

    unified_handler(context, request_context, data).await
}
