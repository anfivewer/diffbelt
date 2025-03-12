use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::common::{
    IsByteArray, KeyValueUpdate, KeyValueUpdateNewOptions, OwnedCollectionKey, OwnedCollectionValue,
};
use crate::context::Context;
use crate::http::constants::PUT_MANY_REQUEST_MAX_BYTES;
use crate::http::data::bytes_generation_id::{
    flatbuffers_generation_id_to_owned_generation_id_opt,
    flatbuffers_phantom_id_to_owned_phantom_id_opt,
};
use crate::http::data::encoded_generation_id::{
    encoded_generation_id_data_decode_opt, encoded_generation_id_data_encode,
};
use crate::http::data::encoded_phantom_id::EncodedPhantomIdJsonDataTrait;
use crate::http::data::key_value_update::KeyValueUpdateJsonDataTrait;
use crate::http::errors::HttpError;
use crate::http::request::request_context::RequestContext;
use crate::http::routing::response::{BaseResponse, BytesVecResponse, HttpResponse};
use crate::http::routing::{HttpHandlerResult, PatternRouteOptions};
use crate::http::util::common_groups::{id_only_group, IdOnlyGroup};
use crate::http::util::encoding::StringDecoder;
use crate::http::util::get_collection::get_collection;
use crate::http::util::read_body::read_limited_body;
use crate::http::util::read_json::read_json;
use crate::http::util::response::create_ok_flatbuffers_response;
use crate::http::validation::{ContentTypeValidation, MethodsValidation};
use crate::util::str_serialization::StrSerializationType;
use diffbelt_macro::fn_box_pin_async;
use diffbelt_protos::protos::api::put_many::PutManyResponseArgs;
use diffbelt_protos::protos::handlers::{ApiHandler, ApiHandlerRequestFlatType, PutManyApiHandler};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::put_many::{PutManyRequestJsonData, PutManyResponseJsonData};
use diffbelt_types::common::phantom_id::EncodedPhantomIdJsonData;
use diffbelt_util::http::read_full_body::FullBody;
use hyper::StatusCode;
use regex::Regex;
use std::sync::Arc;
use tracing::{debug, trace};

pub struct UnifiedRequestData<'a> {
    collection_name: &'a str,
    options: CollectionPutManyOptions,
}

async fn unified_handler(
    context: Arc<Context>,
    request_context: RequestContext,
    data: UnifiedRequestData<'_>,
) -> HttpHandlerResult {
    let collection = get_collection(&context, data.collection_name).await?;

    let result = collection.put_many(data.options).await;

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            debug!(parent: &request_context.span, "finalError {err:?}");
            return match err {
                CollectionMethodError::OutdatedGeneration => Err(HttpError::Custom {
                    status_code: StatusCode::BAD_REQUEST,
                    error: Some("outdatedGeneration"),
                    reason: None,
                    details: None,
                }),
                _ => Err(HttpError::Unspecified),
            };
        }
    };

    if request_context.is_flatbuffers() {
        let mut serializer = Serializer::new();
        let generation_id = Some(serializer.create_vector(result.generation_id.get_byte_array()));
        let response =
            PutManyApiHandler::create_response(serializer, PutManyResponseArgs { generation_id });

        create_ok_flatbuffers_response(response)
    } else {
        let response = PutManyResponseJsonData {
            generation_id: encoded_generation_id_data_encode(
                result.generation_id.as_ref(),
                StrSerializationType::Utf8,
            ),
        };

        let response = serde_json::to_vec(&response).or(Err(HttpError::PublicInternal500(
            "result serialization failed",
        )))?;

        Ok(HttpResponse::BytesVec(BytesVecResponse {
            base: BaseResponse {
                content_type: "application/json; charset=utf-8",
                ..Default::default()
            },
            bytes: response,
        }))
    }
}

#[fn_box_pin_async]
async fn json_handler(options: PatternRouteOptions<IdOnlyGroup>) -> HttpHandlerResult {
    let context = options.context;
    let request = options.request;
    let request_context = options.request_context;
    let collection_name = options.groups.0;

    request.allow_only_methods(&["POST"])?;
    request.allow_only_utf8_json_by_default()?;

    let body = read_limited_body(request, PUT_MANY_REQUEST_MAX_BYTES).await?;
    let options = parse_options_from_json(body)?;

    let options = UnifiedRequestData {
        collection_name: &collection_name,
        options,
    };

    unified_handler(context, request_context, options).await
}

fn parse_options_from_flatbuffers<'a>(
    request: ApiHandlerRequestFlatType<'a, PutManyApiHandler>,
) -> Result<CollectionPutManyOptions, HttpError> {
    let generation_id =
        flatbuffers_generation_id_to_owned_generation_id_opt(request.generation_id())?;
    let phantom_id = flatbuffers_phantom_id_to_owned_phantom_id_opt(request.phantom_id())?;

    let items_fb = request.items().unwrap_or_default();
    let mut items = Vec::with_capacity(items_fb.len());

    for item in items_fb {
        let key = item
            .key()
            .ok_or_else(|| HttpError::GenericFlatbuffers400("no items key"))?;
        let key = OwnedCollectionKey::from_boxed_slice(Box::from(key.bytes()))
            .map_err(|()| HttpError::GenericFlatbuffers400("invalid items key"))?;
        let value = match item.value() {
            None => None,
            Some(value) => Some(OwnedCollectionValue::new(value.bytes())),
        };

        let update = KeyValueUpdate::new(KeyValueUpdateNewOptions {
            key,
            value,
            if_not_present: item.if_not_present(),
        });
        items.push(update);
    }

    let options = CollectionPutManyOptions {
        items,
        generation_id,
        phantom_id,
    };

    Ok(options)
}

fn parse_options_from_json(body: FullBody) -> Result<CollectionPutManyOptions, HttpError> {
    let data: PutManyRequestJsonData = read_json(body)?;

    let decoder = StringDecoder::new(StrSerializationType::Utf8);

    let generation_id = encoded_generation_id_data_decode_opt(data.generation_id)?;
    let phantom_id = EncodedPhantomIdJsonData::decode_opt(data.phantom_id, &decoder)?;

    let mut items = Vec::with_capacity(data.items.len());

    for item in data.items {
        let update = item.deserialize(&decoder)?;
        items.push(update);
    }

    let options = CollectionPutManyOptions {
        items,
        generation_id,
        phantom_id,
    };

    Ok(options)
}

pub fn register_put_many_route(context: &mut Context) {
    context.routing.add_pattern_route(
        Regex::new("^/collections/(?P<id>[^/]+)/putMany$").unwrap(),
        id_only_group,
        json_handler,
    );
}

pub async fn put_many_flatbuffers_route<'a>(
    context: Arc<Context>,
    request_context: RequestContext,
    request: ApiHandlerRequestFlatType<'a, PutManyApiHandler>,
) -> Result<HttpResponse, HttpError> {
    let collection_name = request
        .collection_name()
        .ok_or_else(|| HttpError::GenericFlatbuffers400("no collection name"))?;

    let options = parse_options_from_flatbuffers(request)?;
    unified_handler(
        context,
        request_context,
        UnifiedRequestData {
            collection_name,
            options,
        },
    )
    .await
}
