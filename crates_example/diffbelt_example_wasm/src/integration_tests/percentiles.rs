use alloc::string::String;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::integration_tests::{report_single_test_error, IntegrationTest};

struct PercentilesIntegrationTest;

impl IntegrationTest for PercentilesIntegrationTest {
    #[export_name = "calculatePercentilesEmptyIntegrationTest"]
    extern "C" fn test() -> ErrorCode {
        //

        report_single_test_error(String::from("some error message"));
        ErrorCode::Ok
    }
}
