use crate::types::{ParsedLogLinesKey, ParsedLogLinesValue};
use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput, AggregateReduceInput,
    AggregateTargetInfo,
};
use diffbelt_wasm_binding::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::ptr::slice::SliceRawParts;
use diffbelt_wasm_binding::transform::aggregate::Aggregate;

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
        buffer: FlatbufferAnnotated<
            *mut BytesVecRawParts,
            Annotated<AggregateMapMultiOutput, (TargetKey, MappedValue)>,
        >,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "aggregateInitialAccumulator"]
    extern "C" fn initial_accumulator(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        accumulator: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "aggregateReduce"]
    extern "C" fn reduce(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        input: Annotated<BytesSlice, Annotated<AggregateReduceInput, MappedValue>>,
        accumulator: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "aggregateMergeAccumulators"]
    extern "C" fn merge_accumulators(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        input: SliceRawParts<Annotated<BytesSlice, Accumulator>>,
        accumulator: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    #[export_name = "aggregateApply"]
    extern "C" fn apply(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        accumulator: Annotated<BytesSlice, Accumulator>,
        output: FlatbufferAnnotated<
            *mut BytesVecRawParts,
            Annotated<AggregateApplyOutput, TargetValue>,
        >,
    ) -> ErrorCode {
        todo!()
    }
}
