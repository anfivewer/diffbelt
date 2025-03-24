use crate::integration_tests::query_intermediate::query_intermediate;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use chrono::NaiveDateTime;
use core::fmt::Write;
use core::str::from_utf8;
use diffbelt_example_protos::protos::impls::UpdateMsIntermediateProto;
use diffbelt_protos::align_util::AlignedBytes;
use diffbelt_protos::protos::api::common::{KeyValueUpdate, KeyValueUpdateArgs};
use diffbelt_protos::protos::api::generation::GenerationIdStreamRequestArgs;
use diffbelt_protos::protos::api::put_many::PutManyRequestArgs;
use diffbelt_protos::protos::handlers::{
    ApiHandler, GenerationIdStreamApiHandler, PutManyApiHandler,
};
use diffbelt_protos::{deserialize, Serializer};
use diffbelt_util_no_std::cast::{checked_usize_to_i64, i32_to_f32};
use diffbelt_wasm_binding::cli::run_transform;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::integration_tests::{report_single_test_error, IntegrationTest};
use diffbelt_wasm_binding::requests::Request;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

struct PercentilesIntegrationTest;

#[derive(Debug)]
struct ExpectedIntermediateItem {
    key: Box<str>,
    update_type: Box<str>,
    ms: f32,
}

impl PartialEq for ExpectedIntermediateItem {
    fn eq(&self, other: &Self) -> bool {
        if !(&self.key == &other.key && &self.update_type == &other.update_type) {
            return false;
        }

        let diff = self.ms - other.ms;
        diff < 0.001
    }
}

impl IntegrationTest for PercentilesIntegrationTest {
    #[export_name = "calculatePercentilesEmptyIntegrationTest"]
    unsafe extern "C" fn test() -> ErrorCode {
        let mut serializer = Serializer::new();
        let mut items = Vec::new();

        let mut key = String::new();
        let mut expected_key = String::new();
        const UPDATE_TYPES: [&'static str; 4] = ["first", "second", "third", "four"];
        let mut rnd = ChaCha8Rng::seed_from_u64(0x9a9ddd206ce854ef);

        let mut expected_intermediate_items = Vec::new();

        for i in 0usize..1000 {
            // S 2023-02-20T21:42:48.822Z.000 worker258688:middlewares handleFull updateType:edited_message ms:27.42
            key.clear();
            let date = NaiveDateTime::from_timestamp_millis(
                1741803480953 + checked_usize_to_i64(i) * 24329,
            )
            .expect("cannot create date");
            let day_date = date.format("%Y-%m-%d").to_string();
            let date_rest = date.format("T%H:%M:%S%.3fZ").to_string();
            key.write_fmt(format_args!("S {day_date}{date_rest}.{:0>3} ", i % 113))
                .expect("write");
            key.write_fmt(format_args!("worker{}:middlewares handleFull ", i % 29))
                .expect("write");
            let update_type = UPDATE_TYPES[i % UPDATE_TYPES.len()];
            key.write_fmt(format_args!("updateType:{update_type} ",))
                .expect("write");
            let ms = i32_to_f32(rnd.gen_range(0..100)) + rnd.gen::<f32>();
            key.write_fmt(format_args!("ms:{ms:.3}",)).expect("write");

            let ms = libm::roundf(ms * 1000f32) / 1000f32;

            if i == 0 {
                assert_eq!(key.as_str(), "S 2025-03-12T18:18:00.953Z.000 worker0:middlewares handleFull updateType:first ms:85.583");
            }

            // 2025-03-12 000000007.6 2025-03-12T19:56:45.778Z.025 worker25
            let mut expected_key = String::new();
            expected_key
                .write_fmt(format_args!(
                    "{day_date} {ms:0>11.1} {day_date}{date_rest}.{:0>3} worker{}",
                    i % 113,
                    i % 29
                ))
                .expect("write");

            if i == 0 {
                assert_eq!(
                    expected_key.as_str(),
                    "2025-03-12 000000085.6 2025-03-12T18:18:00.953Z.000 worker0"
                );
            }

            expected_intermediate_items.push(ExpectedIntermediateItem {
                key: Box::from(expected_key.as_str()),
                update_type: Box::from(update_type),
                ms,
            });

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

        expected_intermediate_items.sort_by(|a, b| a.key.cmp(&b.key));

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
        // debug_print("finish parse_lines");
        let () = run_transform("parsed_lines_1d").expect("transform");
        // debug_print("finish parsed_lines_1d");
        let () = run_transform("updateMs_1d_intermediate").expect("transform");
        // debug_print("end transforms");

        let mut buffer_holder = Some(buffer.into_underlying_vec());
        let mut buffer_holder2 = Some(buffer2.into_underlying_vec());
        let mut alignment_vec = Vec::new();
        let mut expected_iter = expected_intermediate_items.iter();

        query_intermediate(&mut buffer_holder, &mut buffer_holder2, |kv| {
            let key = kv.key().expect("no key");
            let key = from_utf8(key.bytes()).expect("key is not string");

            let value = kv.value().expect("no value").bytes();
            let aligned =
                AlignedBytes::ensure_alignment_or_copy(value, &mut alignment_vec).expect("align");
            let value = deserialize::<UpdateMsIntermediateProto>(aligned).expect("parsing");

            let update_type = value.update_type().expect("no update_type");
            let ms = value.ms();

            let item = ExpectedIntermediateItem {
                key: Box::from(key),
                update_type: Box::from(update_type),
                ms,
            };

            let expected_item = expected_iter.next();

            assert_eq!(
                Some(&item),
                expected_item,
                "left is actual, right is expected"
            );
        });

        assert!(expected_iter.next().is_none(), "there is extra items");

        let () = run_transform("updateMs_1d_percentiles").expect("transform");

        report_single_test_error(String::from("some error message3"));
        ErrorCode::Ok
    }
}
