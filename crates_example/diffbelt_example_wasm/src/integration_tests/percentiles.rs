use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use chrono::NaiveDateTime;
use core::fmt::Write;
use diffbelt_protos::protos::api::common::{KeyValueUpdate, KeyValueUpdateArgs};
use diffbelt_protos::protos::api::generation::{
    GenerationIdStreamRequestArgs, StartGenerationRequestArgs, StartGenerationResponse,
};
use diffbelt_protos::protos::api::put_many::PutManyRequestArgs;
use diffbelt_protos::protos::handlers::{
    ApiHandler, GenerationIdStreamApiHandler, PutManyApiHandler, StartGenerationApiHandler,
};
use diffbelt_protos::Serializer;
use diffbelt_util_no_std::cast::{checked_usize_to_i64, i32_to_f32};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::integration_tests::{report_single_test_error, IntegrationTest};
use diffbelt_wasm_binding::requests::Request;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use diffbelt_wasm_binding::cli::{run_transform, sleep_ms};
use diffbelt_wasm_binding::debug_print;

struct PercentilesIntegrationTest;

impl IntegrationTest for PercentilesIntegrationTest {
    #[export_name = "calculatePercentilesEmptyIntegrationTest"]
    extern "C" fn test() -> ErrorCode {
        // let mut serializer = Serializer::new();
        // let collection_name = Some(serializer.create_string("parsed-log-lines"));
        // let generation_id = Some(serializer.create_vector("01".as_bytes()));
        // let mut request = Request::<StartGenerationApiHandler>::call(
        //     serializer,
        //     StartGenerationRequestArgs {
        //         collection_name,
        //         generation_id,
        //         abort_outdated: false,
        //     },
        // )
        // .expect("start generation request");
        //
        // let response = request
        //     .on_request_finished()
        //     .expect("start generation on_request_finished");
        // let _: StartGenerationResponse = response
        //     .response()
        //     .expect("start generation response parsing");

        let mut serializer = Serializer::new();
        let mut items = Vec::new();

        let mut key = String::new();
        const UPDATE_TYPES: [&'static str; 4] = ["first", "second", "third", "four"];
        let mut rnd = ChaCha8Rng::seed_from_u64(0x9a9ddd206ce854ef);

        for i in 0usize..1000 {
            // S 2023-02-20T21:42:48.822Z.000 worker258688:middlewares handleFull updateType:edited_message ms:27.42
            key.clear();
            let date = NaiveDateTime::from_timestamp_millis(
                1741803480953 + checked_usize_to_i64(i) * 10177,
            )
            .expect("cannot create date");
            let date = date.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            key.write_fmt(format_args!("S {date}.{:0>3} ", i % 113)).expect("write");
            key.write_fmt(format_args!("worker{i}:middlewares handleFull "))
                .expect("write");
            key.write_fmt(format_args!(
                "updateType:{} ",
                UPDATE_TYPES[i % UPDATE_TYPES.len()]
            ))
            .expect("write");
            key.write_fmt(format_args!(
                "ms:{:.3}",
                i32_to_f32(rnd.gen_range(0..100)) + rnd.gen::<f32>()
            ))
            .expect("write");

            if i == 0 {
                assert_eq!(key.as_str(), "S 2025-03-12T18:18:00.953Z.000 worker0:middlewares handleFull updateType:first ms:85.583");
            }

            let key = Some(serializer.create_vector(key.as_bytes()));
            let value = Some(serializer.create_vector(&[0u8; 0]));

            items.push(KeyValueUpdate::create(
                serializer.buffer_builder(),
                &KeyValueUpdateArgs {
                    key,
                    value,
                    if_not_present: false,
                },
            ));
        }

        let items = Some(serializer.create_vector(&items));
        let collection_name = Some(serializer.create_string("log-lines"));
        let mut request = Request::<PutManyApiHandler>::call(
            serializer,
            PutManyRequestArgs {
                collection_name,
                items,
                generation_id: None,
                phantom_id: None,
            },
        )
        .expect("request");

        let response = request.on_request_finished().expect("request");
        let response = response.response().expect("request");

        let generation_id = response.generation_id().expect("no generation id").bytes();

        let mut serializer = Serializer::new();
        let collection_name = Some(serializer.create_string("log-lines"));
        let to_generation_id = Some(serializer.create_vector(generation_id));

        let request_serialized = GenerationIdStreamApiHandler::create_request(
            serializer,
            GenerationIdStreamRequestArgs {
                collection_name,
                generation_id: None,
                to_generation_id,
            },
        );

        loop {
            let mut request =
                Request::<GenerationIdStreamApiHandler>::call_raw(request_serialized.as_ref())
                    .expect("request");
            let response = request.on_request_finished().expect("request");
            let response = response.response().expect("request");

            let id = response.generation_id().expect("no id").bytes();

            if id >= generation_id {
                break;
            }
        }

        let buffer = request_serialized.into_aligned_bytes();
        let buffer2 = request.take_buffer().unwrap_or_default();

        let () = run_transform("parse_lines").expect("transform");
        debug_print("finish parse_lines");
        sleep_ms(500);
        let () = run_transform("parsed_lines_1d").expect("transform");
        debug_print("finish parsed_lines_1d");
        sleep_ms(500);
        let () = run_transform("updateMs_1d_intermediate").expect("transform");
        debug_print("end transforms");
        sleep_ms(500);

        report_single_test_error(String::from("some error message3"));
        ErrorCode::Ok
    }
}
