use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use diffbelt_example_protos::protos::log_line::{
    LogTypeWithCount, LogTypeWithCountArgs, ParsedLogLine1d, ParsedLogLine1dArgs,
};
use diffbelt_protos::{deserialize, SerializedRawParts, Serializer};
use diffbelt_util_no_std::bytes::{read_u32_be, write_u32_be};
use diffbelt_util_no_std::cast::{try_usize_to_u32, u32_to_usize};
use diffbelt_wasm_binding::debug_print;

pub struct DayAccumulator<'a> {
    pub total_count: i64,
    pub log_types: BTreeMap<Cow<'a, str>, i64>,
}

impl DayAccumulator<'_> {
    pub fn read_buffer_meta_data(bytes: &[u8]) -> (u32, u32) {
        let accumulator_tail = &bytes[(bytes.len() - 8)..];

        let head = read_u32_be(accumulator_tail);
        let len = read_u32_be(&accumulator_tail[4..]);

        (head, len)
    }

    pub fn parsed_log_line_1d_from_bytes(bytes: &[u8]) -> ParsedLogLine1d<'_> {
        let (head, len) = Self::read_buffer_meta_data(bytes);

        let serialized = &bytes[u32_to_usize(head)..u32_to_usize(head + len)];
        let serialized =
            deserialize::<ParsedLogLine1d>(serialized).expect("cannot parse accumulator");

        serialized
    }

    pub fn from_accumulator_bytes(bytes: &[u8]) -> Self {
        let serialized = Self::parsed_log_line_1d_from_bytes(bytes);
        Self::from_parsed_log_lines_1d(serialized)
    }

    pub fn from_parsed_log_lines_1d(serialized: ParsedLogLine1d<'_>) -> Self {
        let mut accumulator = Self {
            total_count: 0,
            log_types: BTreeMap::new(),
        };

        accumulator.update_with_parsed_log_lines_1d(serialized);

        accumulator
    }

    pub fn update_with_parsed_log_lines_1d(&mut self, serialized: ParsedLogLine1d<'_>) {
        let Some(log_types) = serialized.log_types() else {
            return;
        };

        self.total_count += serialized.count();

        for item in log_types {
            let name = item.name().expect("item name is empty");
            let count = item.count();

            let stored_count = match self.log_types.get_mut(name) {
                None => {
                    self.log_types.insert(Cow::Owned(String::from(name)), 0);
                    self.log_types.get_mut(name).expect("just inserted")
                }
                Some(count) => count,
            };

            *stored_count += count;
        }
    }

    pub fn update_log_type(&mut self, name: &str, is_add: bool) {
        let count = match self.log_types.get_mut(name) {
            None => {
                self.log_types.insert(Cow::Owned(String::from(name)), 0);
                self.log_types.get_mut(name).expect("just inserted")
            }
            Some(count) => count,
        };

        if is_add {
            *count += 1;
            self.total_count += 1;
        } else {
            *count -= 1;
            self.total_count -= 1;
        }
    }

    pub fn serialize_flatbuffer(&self, buffer: Vec<u8>) -> (Vec<u8>, usize, usize) {
        let mut serializer = Serializer::from_vec(buffer);
        let mut items = Vec::with_capacity(self.log_types.len());

        for (name, count) in &self.log_types {
            let count = *count;

            if count == 0 {
                continue;
            }

            let name = serializer.create_string(name.as_ref());

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
                count: self.total_count,
                log_types: Some(log_types),
            },
        );

        let SerializedRawParts {
            mut buffer,
            head,
            len,
        } = serializer.finish(result).into_owned().into_raw_parts();

        (buffer, head, len)
    }

    pub fn serialize_to_buffer(&self, buffer: Vec<u8>) -> Vec<u8> {
        let (mut buffer, head, len) = self.serialize_flatbuffer(buffer);

        buffer.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0]);

        let buffer_len = buffer.len();
        let buffer_tail = &mut buffer[(buffer_len - 8)..];

        write_u32_be(buffer_tail, try_usize_to_u32(head).expect("head too big"));
        write_u32_be(
            &mut buffer_tail[4..],
            try_usize_to_u32(len).expect("len too big"),
        );

        buffer
    }

    pub fn is_valid(&self) -> bool {
        if self.total_count < 0 {
            return false;
        }

        let mut real_total_count = 0;

        for (_, count) in &self.log_types {
            let count = *count;

            if count <= 0 {
                return false;
            }

            real_total_count += count;
        }

        if real_total_count != self.total_count {
            return false;
        }

        true
    }

    pub fn is_empty(&self) -> bool {
        self.total_count == 0
    }
}
