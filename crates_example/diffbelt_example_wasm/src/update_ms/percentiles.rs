use diffbelt_example_protos::protos::impls::UpdateMsAccumulatorProto;
use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput, AggregateReduceInput,
    AggregateTargetInfo,
};
use diffbelt_protos::Serializer;
use diffbelt_wasm_binding::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::ptr::slice::SliceRawParts;
use diffbelt_wasm_binding::transform::aggregate::Aggregate;

struct UpdateMsDayPercentiles;

type SourceKey = ();
type SourceValue = ();
type MappedValue = ();
type Accumulator = (u32, u32, UpdateMsAccumulatorProto);
type TargetKey = ();
type TargetValue = ();

impl Aggregate<SourceKey, SourceValue, MappedValue, Accumulator, TargetKey, TargetValue>
    for UpdateMsDayPercentiles
{
    unsafe extern "C" fn map(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            Annotated<AggregateMapMultiInput, (SourceKey, SourceValue)>,
            Annotated<AggregateMapMultiOutput, (TargetKey, MappedValue)>,
        >,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode {
        todo!()
    }

    unsafe extern "C" fn initial_accumulator(
        target_info: FlatbufferAnnotated<
            BytesSlice,
            Annotated<AggregateTargetInfo, (TargetKey, TargetValue)>,
        >,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode { unsafe {
        let buffer = (&*accumulator_ptr.value).into_empty_vec();
        
        let serializer = Serializer::<UpdateMsAccumulatorProto>::from_vec(buffer);
        
        todo!()
    }}

    unsafe extern "C" fn reduce(
        input: Annotated<BytesSlice, Annotated<AggregateReduceInput, MappedValue>>,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    unsafe extern "C" fn merge_accumulators(
        input: SliceRawParts<Annotated<BytesVecRawParts, Accumulator>>,
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
    ) -> ErrorCode {
        todo!()
    }

    unsafe extern "C" fn apply(
        accumulator_ptr: Annotated<*mut BytesVecRawParts, Accumulator>,
        output: FlatbufferAnnotated<*mut BytesSlice, Annotated<AggregateApplyOutput, TargetValue>>,
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode {
        todo!()
    }
}
