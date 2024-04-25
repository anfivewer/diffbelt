use diffbelt_wasm_binding::annotations::{Annotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};

struct ParsedLogLines1dKv;

impl HumanReadable for ParsedLogLines1dKv {
    #[export_name = "parsedLogLines1dKeyToBytes"]
    extern "C" fn human_readable_key_to_bytes(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        _uffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "parsedLogLines1dBytesToKey"]
    extern "C" fn bytes_to_human_readable_key(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[export_name = "parsedLogLines1dValueToBytes"]
    extern "C" fn human_readable_value_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        todo!("parsedLogLines1dValueToBytes")
    }

    #[export_name = "parsedLogLines1dBytesToValue"]
    extern "C" fn bytes_to_human_readable_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        todo!("parsedLogLines1dBytesToValue")
    }
}
