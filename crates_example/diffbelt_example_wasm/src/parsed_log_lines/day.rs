use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use core::str::from_utf8;

use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput,
    AggregateMapMultiOutputArgs, AggregateMapOutput, AggregateMapOutputArgs, AggregateReduceInput,
    AggregateTargetInfo,
};
use diffbelt_protos::{deserialize, Serializer};
use diffbelt_util_no_std::cast::u8_to_char;
use diffbelt_wasm_binding::annotations::serializer::InputAnnotated;
use diffbelt_wasm_binding::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::ptr::slice::SliceRawParts;
use diffbelt_wasm_binding::transform::aggregate::Aggregate;
use diffbelt_wasm_binding::Regex;

use crate::types::{ParsedLogLinesKey, ParsedLogLinesValue};

struct ParsedLogLinesDay;

type SourceKey<'a> = ParsedLogLinesKey<'a>;
type SourceValue<'a> = ParsedLogLinesValue<'a>;
type MappedValue<'a> = &'a str;
type Accumulator = ();
type TargetKey<'a> = &'a str;
type TargetValue<'a> = &'a ();

impl<'t>
    Aggregate<
        SourceKey<'t>,
        SourceValue<'t>,
        MappedValue<'t>,
        Accumulator,
        TargetKey<'t>,
        TargetValue<'t>,
    > for ParsedLogLinesDay
{
    #[export_name = "aggregateMap"]
    extern "C" fn map(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            Annotated<AggregateMapMultiInput, (SourceKey, SourceValue)>,
            Annotated<AggregateMapMultiOutput, (TargetKey, MappedValue)>,
        >,
        buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        let buffer = unsafe { (*buffer).into_empty_vec() };
        let mut serializer = Serializer::from_vec(buffer);

        let input = unsafe { input_and_output.deserialize() };

        let mut mapped_value = String::new();
        let mut map_outputs_wip = Vec::new();

        for source in input.items().expect("no items") {
            let key = source.source_key().expect("no source key");
            let key = from_utf8(key.bytes()).expect("source key is not utf8");
            let old_value = source.source_old_value().map(|x| x.bytes());
            let new_value = source.source_new_value().map(|x| x.bytes());

            lazy_static::lazy_static! {
                static ref DATE_RE: Regex = Regex::new(r"^(\d\d\d\d-\d\d-\d\d)T.+$").expect("Cannot build DATE_RE");
            }

            let target_key = DATE_RE
                .replace_one(key, "$1")
                .expect("DATE_RE regexp error");

            let target_key = serializer.create_vector(target_key.as_bytes());

            mapped_value.clear();

            let has_old = map_source_value(old_value, false, false, &mut mapped_value);
            map_source_value(new_value, true, has_old, &mut mapped_value);

            let mapped_value = serializer.create_vector(mapped_value.as_bytes());

            let map_output = AggregateMapOutput::create(
                serializer.buffer_builder(),
                &AggregateMapOutputArgs {
                    target_key: Some(target_key),
                    mapped_value: Some(mapped_value),
                },
            );

            map_outputs_wip.push(map_output);
        }

        let map_outputs_wip = serializer.create_vector(&map_outputs_wip);

        let result = AggregateMapMultiOutput::create(
            serializer.buffer_builder(),
            &AggregateMapMultiOutputArgs {
                items: Some(map_outputs_wip),
            },
        );

        let result = serializer.finish(result);

        unsafe {
            input_and_output.value.replace(result.as_bytes().into());
        }

        ErrorCode::Ok
    }

    #[export_name = "aggregateInitialAccumulator"]
    extern "C" fn initial_accumulator(
        _target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        _accumulator: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "aggregateReduce"]
    extern "C" fn reduce(
        _target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        _input: Annotated<BytesSlice, Annotated<AggregateReduceInput, MappedValue>>,
        _accumulator: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "aggregateMergeAccumulators"]
    extern "C" fn merge_accumulators(
        _target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        _input: SliceRawParts<Annotated<BytesSlice, Accumulator>>,
        _accumulator: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "aggregateApply"]
    extern "C" fn apply(
        _target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        _accumulator: Annotated<BytesSlice, Accumulator>,
        _output: FlatbufferAnnotated<
            *mut BytesVecRawParts,
            Annotated<AggregateApplyOutput, TargetValue>,
        >,
    ) -> ErrorCode {
        todo!()
    }
}

fn map_source_value(
    value: Option<&[u8]>,
    is_new: bool,
    has_old: bool,
    output: &mut impl Write,
) -> bool {
    let Some(value) = value else {
        return false;
    };

    let value = deserialize::<SourceValue>(value).expect("invalid source value");

    if has_old {
        () = output.write_char('\n').expect("cannot write");
    }

    () = output
        .write_char(if is_new { '+' } else { '-' })
        .expect("cannot write");

    lazy_static::lazy_static! {
        static ref LOGGER_KEY_RE: Regex = Regex::new(r"^(.*?)\d+(:.*)?$").expect("Cannot build LOGGER_KEY_RE");
    }

    let logger_key = LOGGER_KEY_RE
        .replace_one(value.logger_key().expect("no log key"), "$1#$2")
        .expect("logger_key regexp");

    () = output.write_str(logger_key.as_ref()).expect("cannot write");
    () = output.write_str("::").expect("cannot write");
    () = output
        .write_str(value.log_key().expect("no log key"))
        .expect("cannot write");
    () = output.write_str("::").expect("cannot write");

    let log_level = u8_to_char(value.log_level());

    () = output.write_char(log_level).expect("cannot write");

    true
}
