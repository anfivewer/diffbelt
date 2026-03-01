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
    unsafe extern "C" fn map(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            Annotated<AggregateMapMultiInput, (SourceKey, SourceValue)>,
            Annotated<AggregateMapMultiOutput, (TargetKey, MappedValue)>,
        >,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;

    unsafe extern "C" fn initial_accumulator(
        target_info_ptr: FlatbufferAnnotated<*const u8, Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>>,
        target_info_len: u32,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    unsafe extern "C" fn reduce(
        input_ptr: FlatbufferAnnotated<*const u8, Annotated<AggregateReduceInput, MappedValue>>,
        input_len: u32,
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
    unsafe extern "C" fn merge_accumulators(
        input_ptr: Annotated<*const BytesVecRawParts, Accumulator>,
        input_len: u32,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode;

    unsafe extern "C" fn apply(
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
        output: FlatbufferAnnotated<*mut BytesSlice, Annotated<AggregateApplyOutput, TargetValue>>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode;
}
