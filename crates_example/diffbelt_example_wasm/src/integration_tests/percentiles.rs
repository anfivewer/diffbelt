use alloc::string::String;
use diffbelt_protos::protos::api::generation::{
    StartGenerationRequestArgs, StartGenerationResponse,
};
use diffbelt_protos::protos::api::put_many::PutManyRequestArgs;
use diffbelt_protos::protos::handlers::{PutManyApiHandler, StartGenerationApiHandler};
use diffbelt_protos::Serializer;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::integration_tests::{report_single_test_error, IntegrationTest};
use diffbelt_wasm_binding::requests::Request;

struct PercentilesIntegrationTest;

impl IntegrationTest for PercentilesIntegrationTest {
    #[export_name = "calculatePercentilesEmptyIntegrationTest"]
    extern "C" fn test() -> ErrorCode {
        let mut serializer = Serializer::new();
        let collection_name = Some(serializer.create_string("parsed-log-lines"));
        let generation_id = Some(serializer.create_vector("01".as_bytes()));
        let mut request = Request::<StartGenerationApiHandler>::call(
            serializer,
            StartGenerationRequestArgs {
                collection_name,
                generation_id,
                abort_outdated: false,
            },
        )
        .expect("start generation request");

        let response = request
            .on_request_finished()
            .expect("start generation on_request_finished");
        let _: StartGenerationResponse = response
            .response()
            .expect("start generation response parsing");

        // let mut serializer = Serializer::new();
        // let request = Request::<PutManyApiHandler>::call(
        //     serializer,
        //     PutManyRequestArgs {
        //         items: None,
        //         generation_id: None,
        //         phantom_id: None,
        //     },
        // );

        report_single_test_error(String::from("some error message"));
        ErrorCode::Ok
    }
}
