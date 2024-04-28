use alloc::vec::Vec;
use diffbelt_example_protos::protos::log_line::{
    LogTypeWithCount, LogTypeWithCountArgs, ParsedLogLine1d, ParsedLogLine1dArgs,
};
use diffbelt_protos::{SerializedRawParts, Serializer};
use diffbelt_wasm_binding::annotations::{Annotated, InputOutputAnnotated};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::human_readable::HumanReadable;
use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::Regex;

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
        buffer_ptr: *mut BytesVecRawParts,
    ) -> ErrorCode {
        lazy_static::lazy_static! {
            static ref COUNT_RE: Regex = Regex::new(r"^count: (\d+)\n").expect("Cannot build COUNT_RE");
            static ref ITEMS_RE: Regex = Regex::new(r"^items:\n").expect("Cannot build ITEMS_RE");
            static ref ITEM_RE: Regex = Regex::new(r"^  (.*): (\d+)\n").expect("Cannot build ITEM_RE");
        }

        let buffer = unsafe { (*buffer_ptr).into_empty_vec() };
        let mut serializer = Serializer::from_vec(buffer);

        let input = unsafe { (*input_and_output.value).as_str().expect("not a string") };

        let mut captures_holder = Regex::alloc_captures::<3>();

        let captures = COUNT_RE
            .captures(input, &mut captures_holder)
            .expect("No count");
        let count = captures.get(1).expect("capture");
        let total_count = count.parse::<u64>().expect("Cannot parse count");

        let input = &input[captures.get(0).expect("capture").len()..];
        let captures = ITEMS_RE
            .captures(input, &mut captures_holder)
            .expect("No items");
        let input = &input[captures.get(0).expect("capture").len()..];

        let mut items = Vec::new();

        let mut input = input;

        while !input.is_empty() {
            let captures = ITEM_RE
                .captures(input, &mut captures_holder)
                .expect("Not a item");
            input = &input[captures.get(0).expect("capture").len()..];

            let name = captures.get(1).expect("No item name");
            let count = captures.get(2).expect("No item count");
            let count = count.parse::<u64>().expect("Cannot parse count");

            let name = serializer.create_string(name);

            items.push(LogTypeWithCount::create(
                serializer.buffer_builder(),
                &LogTypeWithCountArgs {
                    name: Some(name),
                    count,
                },
            ));
        }

        let log_types = serializer.create_vector(&items);

        let result = ParsedLogLine1d::create(
            serializer.buffer_builder(),
            &ParsedLogLine1dArgs {
                count: total_count,
                log_types: Some(log_types),
            },
        );

        let SerializedRawParts { buffer, head, len } =
            serializer.finish(result).into_owned().into_raw_parts();

        unsafe {
            *input_and_output.value = BytesSlice::from(&buffer[head..(head + len)]);
            *buffer_ptr = BytesVecRawParts::from(buffer);
        }

        ErrorCode::Ok
    }

    #[export_name = "parsedLogLines1dBytesToValue"]
    extern "C" fn bytes_to_human_readable_value(
        input_and_output: InputOutputAnnotated<*mut BytesSlice, &'static [u8], &str>,
        buffer: Annotated<*mut BytesVecRawParts, &str>,
    ) -> ErrorCode {
        todo!("parsedLogLines1dBytesToValue")
    }
}
