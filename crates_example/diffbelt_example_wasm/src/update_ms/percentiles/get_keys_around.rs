use crate::global::take_buffer_for_realign;
use alloc::string::String;
use alloc::vec::Vec;
use diffbelt_aligned_bytes::AlignedBytes;
use diffbelt_example_protos::protos::impls::{UpdateMsAccumulatorProto, UpdateMsPercentilesProto};
use diffbelt_example_protos::protos::update_ms::{
    UpdateMsAccumulator, UpdateMsAccumulatorArgs, UpdateMsAggregateByType,
    UpdateMsAggregateByTypeArgs,
};
use diffbelt_protos::protos::api::get::GetRequestArgs;
use diffbelt_protos::protos::api::get_keys_around::GetKeysAroundRequestArgs;
use diffbelt_protos::protos::handlers::{GetApiHandler, GetKeysAroundApiHandler};
use diffbelt_protos::protos::transform::aggregate::AggregateTargetInfo;
use diffbelt_protos::{OwnedSerialized, Serializer, deserialize};
use diffbelt_util_no_std::option::store_in_option;
use diffbelt_wasm_binding::debug_print_string;
use diffbelt_wasm_binding::requests::Request;

struct PercentilesTemp {
    percentile: f32,
    intermediate_key: String,
    request: Request<GetKeysAroundApiHandler>,
}

pub fn request_initial_accumulator(
    buffer_holder: &mut Option<Vec<u8>>,
    target_info: AggregateTargetInfo<'_>,
) -> OwnedSerialized<UpdateMsAccumulatorProto> {
    let mut request_buffer_holder = None;
    let mut request_buffer_holder2 = None;

    let mut accumulator_serializer =
        Serializer::<UpdateMsAccumulatorProto>::from_vec(buffer_holder.take().unwrap_or_default());
    let mut by_type_items = None;
    let mut percentiles_items = None;

    'target: {
        let mut serializer = Serializer::from_vec(request_buffer_holder.take().unwrap_or_default());
        let collection_name = Some(serializer.create_string("updateMs:1d:p"));
        let key = Some(
            serializer.reserialize_bytes_vector(target_info.target_key().expect("no target key")),
        );
        let generation_id =
            Some(serializer.reserialize_bytes_vector(
                target_info.generation_id().expect("no prev generation id"),
            ));
        let mut request = Request::<GetApiHandler>::call(
            serializer,
            GetRequestArgs {
                collection_name,
                key,
                generation_id,
                phantom_id: None,
            },
        )
        .expect("request");
        let response = request.on_request_finished().expect("request");
        let target = response.response().expect("request");

        let target = target.item();

        let Some(target) = target else {
            break 'target;
        };

        let value = target.value().expect("no target value").bytes();
        let mut realign_buffer = take_buffer_for_realign();
        let bytes = AlignedBytes::ensure_alignment_or_copy(value, realign_buffer.as_mut()).unwrap();
        let target = deserialize::<UpdateMsPercentilesProto>(bytes).expect("parse");
        let target_by_type = target.by_type().unwrap_or_default();
        let target_percentiles = target.percentiles().unwrap_or_default();

        let by_type_items_ref =
            store_in_option(&mut by_type_items, Vec::with_capacity(target_by_type.len()));

        for by_type in target_by_type {
            let key = by_type
                .key()
                .map(|x| accumulator_serializer.create_string(x));
            let item = UpdateMsAggregateByType::create(
                accumulator_serializer.buffer_builder(),
                &UpdateMsAggregateByTypeArgs {
                    key,
                    sum_ms: by_type.sum_ms(),
                    count: by_type.count(),
                },
            );
            by_type_items_ref.push(item);
        }

        let percentiles_items_ref = store_in_option(
            &mut percentiles_items,
            Vec::with_capacity(target_percentiles.len()),
        );

        for percentile in target_percentiles {
            let p = percentile.percentile();
            let intermediate_key = percentile.intermediate_key().expect("no intermediate key");

            let mut serializer =
                Serializer::from_vec(request_buffer_holder2.take().unwrap_or_default());
            let collection_name = Some(serializer.create_string("updateMs:1d:intermediate"));
            let key = Some(serializer.create_vector(intermediate_key.as_bytes()));
            let generation_id = Some(serializer.reserialize_bytes_vector(
                target_info.generation_id().expect("no prev generation id"),
            ));
            let mut request = Request::<GetKeysAroundApiHandler>::call(
                serializer,
                GetKeysAroundRequestArgs {
                    collection_name,
                    key,
                    generation_id,
                    phantom_id: None,
                    require_key_existance: true,
                    limit: 100,
                },
            )
            .expect("request");

            request_buffer_holder2 = request.take_buffer().map(|x| x.into_underlying_vec());

            percentiles_items_ref.push(PercentilesTemp {
                percentile: p,
                intermediate_key: String::from(intermediate_key),
                request,
            });
        }
    }

    let percentiles = {
        let percentiles_items = percentiles_items.expect("no percentiles vec");

        for percentile_temp in percentiles_items {
            let PercentilesTemp {
                percentile,
                intermediate_key,
                mut request,
            } = percentile_temp;

            let response = request.on_request_finished().expect("request");
            let response = response.response().expect("request");

            use alloc::format;
            debug_print_string(format!("response {response:#?}"));
        }
    };

    let by_type = by_type_items.map(|x| accumulator_serializer.create_vector(&x));

    let root = UpdateMsAccumulator::create(
        accumulator_serializer.buffer_builder(),
        &UpdateMsAccumulatorArgs {
            total_count: 0,
            by_type,
            percentiles: None,
        },
    );
    let root = accumulator_serializer.finish(root);

    // root
    
    todo!()
}
