use crate::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use crate::error_code::ErrorCode;
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};
use crate::ptr::slice::SliceRawParts;
use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput, AggregateReduceInput,
    AggregateTargetInfo,
};

pub trait Aggregate<SourceKey, SourceValue, MappedValue, Accumulator, TargetKey, TargetValue> {
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
    ) -> ErrorCode;

    extern "C" fn initial_accumulator(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        accumulator: FlatbufferAnnotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    extern "C" fn reduce(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        input: Annotated<BytesSlice, Annotated<AggregateReduceInput, MappedValue>>,
        accumulator: FlatbufferAnnotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    extern "C" fn merge_accumulators(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        input: SliceRawParts<Annotated<BytesSlice, Accumulator>>,
        accumulator: FlatbufferAnnotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    extern "C" fn apply(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        accumulator: Annotated<BytesSlice, Accumulator>,
        output: FlatbufferAnnotated<*mut BytesVecRawParts, Annotated<AggregateApplyOutput, TargetValue>>,
    ) -> ErrorCode;
}
