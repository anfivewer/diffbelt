use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput, AggregateReduceInput,
    AggregateTargetInfo,
};

use crate::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use crate::error_code::ErrorCode;
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};
use crate::ptr::slice::SliceRawParts;

pub trait Aggregate<
    SourceKey,
    SourceValue,
    MappedValue,
    Accumulator: 'static,
    TargetKey,
    TargetValue,
>
{
    extern "C" fn map(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            Annotated<AggregateMapMultiInput, (SourceKey, SourceValue)>,
            Annotated<AggregateMapMultiOutput, (TargetKey, MappedValue)>,
        >,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;

    extern "C" fn initial_accumulator(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    extern "C" fn reduce(
        input: Annotated<BytesSlice, Annotated<AggregateReduceInput, MappedValue>>,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    /**
     * If this function is absent, aggregate will be fully sequential.
     *
     * This function can not been called if there is only one accumulator
     * (too few items, for example).
     *
     * Accumulator argument receives first accumulator.
     */
    extern "C" fn merge_accumulators(
        input: SliceRawParts<Annotated<BytesSlice, Accumulator>>,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    extern "C" fn apply(
        accumulator: Annotated<*mut BytesVecRawParts, Accumulator>,
        output: FlatbufferAnnotated<*mut BytesSlice, Annotated<AggregateApplyOutput, TargetValue>>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;
}
