use crate::protos::log_line::{
    ParsedLogLine, ParsedLogLine1d, ParsedLogLine1dArgs, ParsedLogLineArgs,
};
use crate::protos::update_ms::{
    UpdateMsAccumulator, UpdateMsAccumulatorArgs, UpdateMsIntermediate, UpdateMsIntermediateArgs,
    UpdateMsPercAcc, UpdateMsPercAccArgs, UpdateMsPercentiles, UpdateMsPercentilesArgs,
};
use diffbelt_protos::flatbuffers_generic;

flatbuffers_generic!(ParsedLogLine, ParsedLogLineArgs<'a>);
flatbuffers_generic!(UpdateMsIntermediate, UpdateMsIntermediateArgs<'a>);
flatbuffers_generic!(UpdateMsPercentiles, UpdateMsPercentilesArgs<'a>);
flatbuffers_generic!(ParsedLogLine1d, ParsedLogLine1dArgs<'a>);
flatbuffers_generic!(UpdateMsAccumulator, UpdateMsAccumulatorArgs<'a>);
flatbuffers_generic!(UpdateMsPercAcc, UpdateMsPercAccArgs<'a>);
