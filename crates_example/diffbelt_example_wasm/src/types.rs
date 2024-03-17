use diffbelt_example_protos::protos::log_line::ParsedLogLine;

pub type LogLinesKey<'a> = &'a str;
pub type LogLinesValue = ();
pub type ParsedLogLinesKey<'a> = &'a str;
pub type ParsedLogLinesValue<'a> = ParsedLogLine<'a>;
