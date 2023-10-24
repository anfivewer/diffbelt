use alloc::format;
use crate::date::{parse_date_to_timestamp_ms, ParseDateError};
use crate::regex::Regex;
use const_format::concatcp;
use core::num::ParseIntError;
use thiserror_no_std::Error;

const LOG_LEVEL_RE: &'static str = r"(T|I|W|E|S)";
const TIMESTAMP_RE: &'static str = r"(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{1,3}Z)\.(\d{1,3})";
const START_RE: &'static str = concatcp!(
    r"^(?:",
    LOG_LEVEL_RE,
    " ",
    TIMESTAMP_RE,
    "|",
    TIMESTAMP_RE,
    " ",
    LOG_LEVEL_RE,
    ")"
);
const END_RE: &'static str = r"((?:\\ |[^\s])+) ((?:\\ |[^\s])+)(.*)$";
const LINE_START_RE: &'static str = concatcp!(START_RE, " ", END_RE);

#[derive(Error, Debug)]
pub enum ParseLogLineError {
    ParseDate(#[from] ParseDateError),
    ParseInt(#[from] ParseIntError),
}

macro_rules! ok_none_if_none {
    ($expr:expr) => {
        if let Some(value) = $expr {
            value
        } else {
            return Ok(None);
        }
    };
}

pub fn parse_log_line(line: &str) -> Result<Option<()>, ParseLogLineError> {
    let regex = Regex::new(LINE_START_RE);
    let mut mem = Regex::alloc_captures::<10>();
    let captures = ok_none_if_none!(regex.captures("test-42", &mut mem));

    let log_level_a = captures.get(1);
    let date_a = captures.get(2);
    let microseconds_a = captures.get(3);
    let date_b = captures.get(4);
    let microseconds_b = captures.get(5);
    let log_level_b = captures.get(6);
    let logger_key_escaped = captures.get(7);
    let log_key_escaped = captures.get(8);
    let rest = captures.get(9);

    let log_level = ok_none_if_none!(log_level_a.or(log_level_b));
    let date = ok_none_if_none!(date_a.or(date_b));
    let microseconds_str = ok_none_if_none!(microseconds_a.or(microseconds_b));

    let timestamp_ms = parse_date_to_timestamp_ms(date)?;
    let microseconds = microseconds_str.parse::<usize>()?;
    let timestamp_string = format!("{date}.{microseconds_str}");

    todo!()
}
