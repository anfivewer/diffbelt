use diffbelt_protos::protos::transform::map_filter::{MapFilterMultiInput, MapFilterMultiOutput};

use crate::annotations::{FlatbufferAnnotated, InputOutputAnnotated};
use crate::error_code::ErrorCode;
use crate::ptr::bytes::{BytesSlice, BytesVecRawParts};

pub trait MapFilter {
    extern "C" fn map_filter(
        input_and_output: InputOutputAnnotated<
            *mut BytesSlice,
            MapFilterMultiInput,
            MapFilterMultiOutput,
        >,
        buffer: FlatbufferAnnotated<*mut BytesVecRawParts, MapFilterMultiOutput>,
    ) -> ErrorCode;
}
