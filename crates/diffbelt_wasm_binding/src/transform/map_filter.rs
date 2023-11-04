use crate::bytes::{BytesSlice, BytesVecRawParts};
use crate::error_code::ErrorCode;
use diffbelt_protos::protos::transform::map_filter::{MapFilterMultiInput, MapFilterMultiOutput};
use diffbelt_util_no_std::comments::Annotated;

pub trait MapFilter {
    extern "C" fn map_filter(
        input_and_output: Annotated<*mut BytesSlice, (MapFilterMultiInput, MapFilterMultiOutput)>,
        buffer: Annotated<*mut BytesVecRawParts, MapFilterMultiOutput>,
    ) -> ErrorCode;
}
