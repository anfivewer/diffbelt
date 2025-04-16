mod constants;
mod get_keys_around;
mod reduce;

use crate::global::take_buffer_for_realign;
use crate::types::{IntermediateKey, UpdateMsPercentilesKey};
use crate::update_ms::percentiles::get_keys_around::request_initial_accumulator;
use crate::update_ms::percentiles::reduce::ReduceAccumulator;
use alloc::vec::Vec;
use core::str::from_utf8;
use diffbelt_aligned_bytes::AlignedBytes;
use diffbelt_example_protos::protos::impls::{
    UpdateMsAccumulatorProto, UpdateMsIntermediateDiffProto, UpdateMsIntermediateProto,
    UpdateMsPercentilesProto,
};
use diffbelt_example_protos::protos::update_ms::{
    UpdateMsIntermediateDiff, UpdateMsIntermediateDiffArgs,
};
use diffbelt_protos::protos::impls::{
    AggregateMapMultiInputProto, AggregateMapMultiOutputProto, AggregateReduceInputProto,
    AggregateTargetInfoProto,
};
use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput,
    AggregateMapMultiOutputArgs, AggregateMapOutput, AggregateMapOutputArgs, AggregateReduceInput,
    AggregateTargetInfo,
};
use diffbelt_protos::{Serializer, Vector, WIPOffset, deserialize};
use diffbelt_wasm_binding::annotations::head_len_vec::FlatbuffersHeadLenVecAnnotation;
use diffbelt_wasm_binding::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::ptr::slice::SliceRawParts;
use diffbelt_wasm_binding::transform::aggregate::Aggregate;
use hashbrown::HashMap;
use regex::Regex;

struct UpdateMsDayPercentiles;

type SourceKey<'a> = IntermediateKey<'a>;
type SourceValue = UpdateMsIntermediateProto;
type MappedValue = UpdateMsIntermediateDiffProto;
type Accumulator = FlatbuffersHeadLenVecAnnotation<UpdateMsAccumulatorProto>;
type TargetKey<'a> = UpdateMsPercentilesKey<'a>;
type TargetValue = UpdateMsPercentilesProto;

impl<'t> Aggregate<SourceKey<'t>, SourceValue, MappedValue, Accumulator, TargetKey<'t>, TargetValue>
    for UpdateMsDayPercentiles
{
    #[unsafe(export_name = "updateMsPercentilesMap")]
    unsafe extern "C" fn map(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            Annotated<AggregateMapMultiInput, (SourceKey, SourceValue)>,
            Annotated<AggregateMapMultiOutput, (TargetKey, MappedValue)>,
        >,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode {
        let mut realign_buffer = take_buffer_for_realign();
        let input = {
            let input = unsafe { (&*input_and_output.value).as_slice() };
            let input = AlignedBytes::ensure_alignment_or_copy(input, realign_buffer.as_mut())
                .expect("input");
            deserialize::<AggregateMapMultiInputProto>(input).expect("input")
        };

        lazy_static::lazy_static! {
            static ref SOURCE_KEY_RE: Regex = Regex::new(r"^([^ ]+) ").expect("Cannot build SOURCE_KEY_RE");
        }

        let mut items_by_day = HashMap::new();

        for item in input.items().unwrap_or_default() {
            // 2025-03-12 000000025.1 2025-03-12T23:46:27.443Z.019 worker27
            let source_key =
                from_utf8(item.source_key().expect("no source_key").bytes()).expect("parsing");

            let Some(captures) = SOURCE_KEY_RE.captures(source_key) else {
                panic!("not matches {source_key}");
            };

            let day = captures.get(1).expect("no group").as_str();

            let entries = match items_by_day.get_mut(day) {
                Some(entries) => entries,
                None => {
                    items_by_day.insert(day, Vec::new());
                    items_by_day.get_mut(day).expect("just inserted")
                }
            };

            entries.push((source_key, item.source_old_value(), item.source_new_value()));
        }

        let mut serializer = {
            let buffer = unsafe { (&*buffer_ptr).into_empty_vec() };
            Serializer::<AggregateMapMultiOutputProto>::from_vec(buffer)
        };

        let mut items = Vec::new();
        let mut buffer = None;

        for (key, values) in items_by_day {
            let target_key = Some(serializer.create_vector(key.as_bytes()));

            for (source_key, old_value, new_value) in values {
                let item_serializer = buffer.take().unwrap_or_default();
                let mut item_serializer =
                    Serializer::<UpdateMsIntermediateDiffProto>::from_vec(item_serializer);

                let mut old_ms = 0f32;
                let mut new_ms = 0f32;
                let source_key = Some(item_serializer.create_vector(source_key.as_bytes()));

                let mut serialize_type = |value: Vector<u8>, is_new: bool| -> WIPOffset<&str> {
                    let mut realign_buffer = take_buffer_for_realign();
                    let value = AlignedBytes::ensure_alignment_or_copy(
                        value.bytes(),
                        realign_buffer.as_mut(),
                    )
                    .expect("align");
                    let value = deserialize::<UpdateMsIntermediateProto>(value).expect("parse");
                    let update_type = value.update_type().expect("no update_type");
                    if is_new {
                        new_ms = value.ms();
                    } else {
                        old_ms = value.ms();
                    }
                    item_serializer.create_string(update_type)
                };

                let old_update_type = old_value.map(|x| serialize_type(x, false));
                let new_update_type = new_value.map(|x| serialize_type(x, true));

                let root = UpdateMsIntermediateDiff::create(
                    item_serializer.buffer_builder(),
                    &UpdateMsIntermediateDiffArgs {
                        key: source_key,
                        old_update_type,
                        new_update_type,
                        old_ms,
                        new_ms,
                    },
                );
                let root = item_serializer.finish(root);

                let mapped_value = Some(serializer.create_vector(root.as_bytes()));

                let item = AggregateMapOutput::create(
                    serializer.buffer_builder(),
                    &AggregateMapOutputArgs {
                        target_key,
                        mapped_value,
                    },
                );

                items.push(item);
            }
        }

        let items = Some(serializer.create_vector(&items));
        let root = AggregateMapMultiOutput::create(
            serializer.buffer_builder(),
            &AggregateMapMultiOutputArgs { items },
        );
        let root = serializer.finish(root);

        unsafe {
            *input_and_output.value = root.as_bytes().into();
            *buffer_ptr = root.into_underlying_buffer().into();
        }

        ErrorCode::Ok
    }

    #[unsafe(export_name = "updateMsPercentilesInitialAccumulator")]
    unsafe extern "C" fn initial_accumulator(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        let buffer = unsafe { (&*accumulator_ptr.value).into_empty_vec() };
        let mut buffer_holder = Some(buffer);
        let mut target_info_realign_buffer = take_buffer_for_realign();

        let target_info = {
            let slice = unsafe { target_info.value.as_slice() };
            let bytes =
                AlignedBytes::ensure_alignment_or_copy(slice, target_info_realign_buffer.as_mut())
                    .expect("realign");
            deserialize::<AggregateTargetInfoProto>(bytes).expect("parse")
        };

        let root = request_initial_accumulator(&mut buffer_holder, target_info);

        unsafe {
            accumulator_ptr.save(root.as_ref());
        }

        ErrorCode::Ok
    }

    #[unsafe(export_name = "updateMsPercentilesReduce")]
    unsafe extern "C" fn reduce(
        input: Annotated<BytesSlice, Annotated<AggregateReduceInput, MappedValue>>,
        mut accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        let accumulator = unsafe { accumulator_ptr.deserialize().expect("parse") };
        let mut realign_buffer = take_buffer_for_realign();
        let input = unsafe {
            let bytes = input.value.as_slice();
            let bytes = AlignedBytes::ensure_alignment_or_copy(bytes, realign_buffer.as_mut())
                .expect("align");
            deserialize::<AggregateReduceInputProto>(bytes).expect("parse")
        };

        let mut accumulator = ReduceAccumulator::new(accumulator);
        let accumulator = accumulator.reduce(input);

        unsafe {
            accumulator_ptr.save(accumulator.as_ref());
        }

        ErrorCode::Ok
    }

    #[unsafe(export_name = "updateMsPercentilesMergeAccumulatorsNotImplemented")]
    unsafe extern "C" fn merge_accumulators(
        _input: SliceRawParts<Annotated<BytesVecRawParts, Accumulator>>,
        _accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        panic!("UpdateMsDayPercentiles cannot be merged");
    }

    #[unsafe(export_name = "updateMsPercentilesMsApply")]
    unsafe extern "C" fn apply(
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
        output: FlatbufferAnnotated<*mut BytesSlice, Annotated<AggregateApplyOutput, TargetValue>>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode {
        todo!()
    }
}
