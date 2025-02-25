use crate::protos::log_line::{ParsedLogLine, ParsedLogLine1d};
use crate::protos::update_ms::{UpdateMsIntermediate, UpdateMsPercentiles};
use diffbelt_protos::flatbuffers_generic;

flatbuffers_generic!(ParsedLogLine, ParsedLogLineProto);
flatbuffers_generic!(UpdateMsIntermediate, UpdateMsIntermediateProto);
flatbuffers_generic!(UpdateMsPercentiles, UpdateMsPercentilesProto);
flatbuffers_generic!(ParsedLogLine1d, ParsedLogLine1dProto);
