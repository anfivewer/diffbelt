use crate::global::take_buffer_for_realign;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use diffbelt_example_protos::protos::impls::UpdateMsPercentilesProto;
use diffbelt_example_protos::protos::update_ms::{
    UpdateMsAggregateByType, UpdateMsAggregateByTypeArgs, UpdateMsIntermediate,
    UpdateMsIntermediateArgs, UpdateMsPerc, UpdateMsPercArgs, UpdateMsPercentiles,
    UpdateMsPercentilesArgs,
};
use diffbelt_protos::align_util::AlignedBytes;
use diffbelt_protos::deserialize;
use diffbelt_wasm_binding::annotations::serializer::IntoSerializerAnnotated;
use diffbelt_wasm_binding::annotations::{Annotated, FlatbufferAnnotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use regex::Regex;

struct UpdateMsPercentilesKv;

mod aggregate;

impl HumanReadable for UpdateMsPercentilesKv {
    #[unsafe(export_name = "updateMsPercentilesKeyToBytes")]
    unsafe extern "C" fn human_readable_key_to_bytes(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        _buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[unsafe(export_name = "updateMsPercentilesBytesToKey")]
    unsafe extern "C" fn bytes_to_human_readable_key(
        _input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        _buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        ErrorCode::Ok
    }

    #[unsafe(export_name = "updateMsPercentilesValueToBytes")]
    unsafe extern "C" fn human_readable_value_to_bytes(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &str, &'static [u8]>,
        buffer: *mut BytesVecRawParts,
    ) -> ErrorCode {
        /*
           sum_ms: 123.456
           by_type:
             - key: some_type
               sum_ms: 4123.12
               count: 124
             - key: another_type
               sum_ms: 4123.12
               count: 124
           percentiles:
             - percentile: 0
               intermediate_key: aaa
             - percentile: 1
               intermediate_key: bbb
        */

        let input = unsafe { (&*input_and_output.value).as_str() }.expect("not a string");

        lazy_static::lazy_static! {
            static ref SUM_MS_RE: Regex = Regex::new(r"^sum_rs: (.+)$").expect("Cannot build SUM_MS_RE");
            static ref BY_TYPE_RE: Regex = Regex::new(r"^by_type:$").expect("Cannot build BY_TYPE_RE");
            static ref BY_TYPE_KEY_RE: Regex = Regex::new(r"^  - key: (.+)$").expect("Cannot build BY_TYPE_KEY_RE");
            static ref BY_TYPE_SUM_MS_RE: Regex = Regex::new(r"^    sum_ms: (.+)$").expect("Cannot build BY_TYPE_SUM_MS_RE");
            static ref BY_TYPE_COUNT_RE: Regex = Regex::new(r"^    count: (\d+)$").expect("Cannot build BY_TYPE_COUNT_RE");
            static ref PERCENTILES_RE: Regex = Regex::new(r"^percentiles:$").expect("Cannot build PERCENTILES_RE");
            static ref PERCENTILES_PERCENTILE_RE: Regex = Regex::new(r"^  - percentile: (.+)$").expect("Cannot build PERCENTILES_PERCENTILE_RE");
            static ref PERCENTILES_INTERMEDIATE_KEY_RE: Regex = Regex::new(r"^    intermediate_key: (.+)$").expect("Cannot build PERCENTILES_INTERMEDIATE_KEY_RE");
        }

        let buffer_ptr = FlatbufferAnnotated::<_, UpdateMsPercentilesProto>::from(buffer);
        let mut serializer_with_ptr = unsafe { buffer_ptr.into_serializer() };
        let serializer = serializer_with_ptr.serializer_mut();

        let mut lines = input.lines().peekable();

        let line = lines.next().expect("no lines");
        let sum_ms = SUM_MS_RE
            .captures(line)
            .and_then(|x| x.get(1))
            .expect("no sum_ms")
            .as_str();
        let sum_ms = sum_ms.parse::<f64>().expect("invalid sum_ms");

        let line = lines.next().expect("no lines2");
        if !BY_TYPE_RE.is_match(line) {
            panic!("no by_type:");
        }

        let mut by_type_items = Vec::new();
        let mut percentile_items = Vec::new();

        loop {
            let line = *lines.peek().expect("no lines3");
            let key = BY_TYPE_KEY_RE.captures(line);

            let key = match key {
                Some(x) => x,
                None => {
                    break;
                }
            };

            let key = key.get(1).expect("no key").as_str();
            let key = serializer.create_string(key);

            lines.next();

            let line = lines.next().expect("no lines4");
            let sum_ms = BY_TYPE_SUM_MS_RE
                .captures(line)
                .and_then(|x| x.get(1))
                .expect("no sum_ms")
                .as_str();
            let sum_ms = sum_ms.parse::<f64>().expect("invalid sum_ms");

            let line = lines.next().expect("no lines5");
            let count = BY_TYPE_COUNT_RE
                .captures(line)
                .and_then(|x| x.get(1))
                .expect("no count")
                .as_str();
            let count = count.parse::<u64>().expect("invalid sum_ms");

            let item = UpdateMsAggregateByType::create(
                serializer.buffer_builder(),
                &UpdateMsAggregateByTypeArgs {
                    key: Some(key),
                    sum_ms,
                    count,
                },
            );
            by_type_items.push(item);
        }

        let line = lines.next().expect("no lines6");
        if !PERCENTILES_RE.is_match(line) {
            panic!("no percentiles:");
        }

        loop {
            let line = lines.next();
            let line = match line {
                Some(x) => x,
                None => {
                    break;
                }
            };

            let percentile = PERCENTILES_PERCENTILE_RE
                .captures(line)
                .and_then(|x| x.get(1))
                .expect("no percentile")
                .as_str();
            let percentile = percentile.parse::<f32>().expect("invalid percentile");

            let line = lines.next().expect("no lines7");
            let intermediate_key = PERCENTILES_INTERMEDIATE_KEY_RE
                .captures(line)
                .and_then(|x| x.get(1))
                .expect("no percentile")
                .as_str();
            let intermediate_key = serializer.create_string(intermediate_key);

            let item = UpdateMsPerc::create(
                serializer.buffer_builder(),
                &UpdateMsPercArgs {
                    percentile,
                    intermediate_key: Some(intermediate_key),
                },
            );
            percentile_items.push(item);
        }

        let by_type = serializer.create_vector(&by_type_items);
        let percentiles = serializer.create_vector(&percentile_items);

        let result = UpdateMsPercentiles::create(
            serializer.buffer_builder(),
            &UpdateMsPercentilesArgs {
                sum_ms,
                by_type: Some(by_type),
                percentiles: Some(percentiles),
            },
        );
        let result = serializer_with_ptr.finish(result);

        unsafe {
            *input_and_output.value = BytesSlice::from(result.serialized_data().value);
            let () = result.save();
        }

        ErrorCode::Ok
    }

    #[unsafe(export_name = "updateMsPercentilesBytesToValue")]
    unsafe extern "C" fn bytes_to_human_readable_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        let mut buffer_holder = take_buffer_for_realign();
        let input = {
            let bytes = unsafe { (&*input_and_output.value).as_slice() };
            let bytes = AlignedBytes::ensure_alignment_or_copy(bytes, buffer_holder.as_mut())
                .expect("align error");
            deserialize::<UpdateMsPercentilesProto>(bytes).expect("deserialization")
        };

        let output = unsafe { (&*buffer.value).into_empty_vec() };
        let mut output = String::from_utf8(output).expect("vector was empty");

        write!(output, "sum_ms: {}\n", input.sum_ms()).expect("writing");
        output.push_str("by_type:\n");

        for by_type in input.by_type().unwrap_or_default() {
            write!(
                output,
                "  - key: {key}\n    sum_ms: {sum_ms}\n    count: {count}\n",
                key = by_type.key().unwrap_or_default(),
                sum_ms = by_type.sum_ms(),
                count = by_type.count(),
            )
            .expect("writing");
        }

        output.push_str("percentiles:\n");

        for percentile in input.percentiles().unwrap_or_default() {
            write!(
                output,
                "  - percentile: {percentile}\n    intermediate_key: {key}\n",
                percentile = percentile.percentile(),
                key = percentile.intermediate_key().unwrap_or_default(),
            )
            .expect("writing");
        }

        unsafe {
            *input_and_output.value = BytesSlice::from(output.as_bytes());
            *buffer.value = BytesVecRawParts::from(output.into_bytes());
        }

        ErrorCode::Ok
    }
}
