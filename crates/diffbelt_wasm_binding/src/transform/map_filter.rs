use diffbelt_protos::protos::impls::{MapFilterMultiInputProto, MapFilterMultiOutputProto};
use diffbelt_protos::protos::transform::map_filter::{MapFilterMultiInput, MapFilterMultiOutput};

use crate::annotations::{FlatbufferAnnotated, InputOutputAnnotated};
use crate::error_code::ErrorCode;
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};

pub trait MapFilter {
    extern "C" fn map_filter(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            MapFilterMultiInputProto,
            MapFilterMultiOutputProto,
        >,
        buffer: FlatbufferAnnotated<*mut BytesVecRawParts, MapFilterMultiOutputProto>,
    ) -> ErrorCode;
}
