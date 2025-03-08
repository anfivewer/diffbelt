use crate::protos::log_line::{ParsedLogLine, ParsedLogLine1d};
use crate::protos::update_ms::{UpdateMsIntermediate, UpdateMsPercentiles};
use diffbelt_protos::flatbuffers_generic;

flatbuffers_generic!(ParsedLogLine);
flatbuffers_generic!(UpdateMsIntermediate);
flatbuffers_generic!(UpdateMsPercentiles);
flatbuffers_generic!(ParsedLogLine1d);
