use alloc::format;
use core::fmt::Debug;
use diffbelt_wasm_binding::debug_print_string;
use diffbelt_wasm_binding::error_code::ErrorCode;

pub fn run_error_coded<E: Debug, F: FnOnce() -> Result<ErrorCode, E>>(f: F) -> ErrorCode {
    let result = f();

    result.map_or_else(
        |err| {
            debug_print_string(format!("{err:?}"));
            ErrorCode::Fail
        },
        |code| code,
    )
}
