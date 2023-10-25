use alloc::borrow::Cow;
use crate::date::{parse_date_to_timestamp_ms, ParseDateError};
use crate::regex::{Regex, RegexError};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use const_format::concatcp;
use core::num::ParseIntError;
use diffbelt_example_protos::protos::log_line::{ParsedLogLine, ParsedLogLineArgs, Prop, PropArgs};
use diffbelt_protos::{OwnedSerialized, Serializer, WIPOffset};
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
const LINE_START_RE_STR: &'static str = concatcp!(START_RE, " ", END_RE);

lazy_static::lazy_static! {
    static ref LINE_START_RE: Regex = Regex::new(LINE_START_RE_STR).expect("Cannot build LINE_START_RE");
    static ref UNESCAPE_KEY_RE: Regex = Regex::new(r"\\(\\|\s)").expect("Cannot build LOGGER_KEY_UNESCAPE_RE");
    static ref UNESCAPE_PROP_KEY_RE: Regex = Regex::new(r"\\(\\|\s|:)").expect("Cannot build UNESCAPE_PROP_KEY_RE");
    static ref UNESCAPE_EXTRA_RE: Regex = Regex::new(r"\\(\\|\|)").expect("Cannot build UNESCAPE_EXTRA_RE");
    static ref LOGGER_KEY_END_RE: Regex = Regex::new(r":.*$").expect("Cannot build LOGGER_KEY_END_RE");
    static ref PROPS_RE: Regex = Regex::new(r"^\s((?:\\ |\\:|[^\s:])+):((?:\\ |[^\s])+)").expect("Cannot build PROPS_RE");
    static ref EXTRA_RE: Regex = Regex::new(r"^\s*\|((?:\\\||[^|])+)").expect("Cannot build EXTRA_RE");
    static ref CR_RE: Regex = Regex::new(r"\\r").expect("Cannot build CR_RE");
    static ref LF_RE: Regex = Regex::new(r"\\n").expect("Cannot build LF_RE");
}

#[derive(Error, Debug)]
pub enum ParseLogLineError {
    ParseDate(#[from] ParseDateError),
    ParseInt(#[from] ParseIntError),
    Regex(#[from] RegexError),
    LogLevelIsEmpty,
    NoPropsKey,
    NoPropsValue,
    NoExtra,
    RestLeft,
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

pub struct LogLineHeader<'a> {
    log_level: &'a str,
    logger_key: Cow<'a, str>,
    logger_key_start: Cow<'a, str>,
    log_key: Cow<'a, str>,
    timestamp_ms: u64,
    microseconds: usize,
    pub log_line_key: String,
    log_line_key_timestamp_string_len: usize,
    rest: &'a str,
}

pub fn parse_log_line_header(line: &str) -> Result<Option<LogLineHeader<'_>>, ParseLogLineError> {
    let mut mem = Regex::alloc_captures::<10>();
    let captures = ok_none_if_none!(LINE_START_RE.captures(line, &mut mem));

    let log_level_a = captures.get(1);
    let date_a = captures.get(2);
    let microseconds_a = captures.get(3);
    let date_b = captures.get(4);
    let microseconds_b = captures.get(5);
    let log_level_b = captures.get(6);
    let logger_key_escaped = ok_none_if_none!(captures.get(7));
    let log_key_escaped = ok_none_if_none!(captures.get(8));
    let rest = ok_none_if_none!(captures.get(9));

    let log_level = ok_none_if_none!(log_level_a.or(log_level_b));
    let date = ok_none_if_none!(date_a.or(date_b));
    let microseconds_str = ok_none_if_none!(microseconds_a.or(microseconds_b));

    let timestamp_ms = parse_date_to_timestamp_ms(date)?;
    let microseconds = microseconds_str.parse::<usize>()?;

    let logger_key = UNESCAPE_KEY_RE.replace_all(logger_key_escaped, "$1")?;
    let logger_key_start = LOGGER_KEY_END_RE.replace_one(logger_key_escaped, "")?;
    let log_key = UNESCAPE_KEY_RE.replace_all(log_key_escaped, "$1")?;

    let log_line_key = format!("{date}.{microseconds_str} {logger_key_start}");
    let log_line_key_timestamp_string_len = date.len() + 1 + microseconds_str.len();

    Ok(Some(LogLineHeader {
        log_level,
        logger_key,
        logger_key_start,
        log_key,
        timestamp_ms,
        microseconds,
        log_line_key,
        log_line_key_timestamp_string_len,
        rest,
    }))
}

impl LogLineHeader<'_> {
    fn timestamp_string(&self) -> &str {
        &self.log_line_key.as_str()[0..self.log_line_key_timestamp_string_len]
    }

    pub fn serialize<'s>(
        &self,
    ) -> Result<OwnedSerialized, ParseLogLineError> {
        let Self {
            log_level,
            logger_key,
            logger_key_start: _,
            log_key,
            timestamp_ms,
            microseconds,
            log_line_key: _,
            log_line_key_timestamp_string_len: _,
            rest,
        } = self;

        let mut rest = *rest;

        let mut mem = Regex::alloc_captures::<3>();
        let mut props = Vec::new();
        let mut extras = Vec::new();

        while let Some(captures) = PROPS_RE.captures(rest, &mut mem) {
            let capture_len = captures.get(0).unwrap().len();
            let key_escaped = captures
                .get(1)
                .ok_or_else(|| ParseLogLineError::NoPropsKey)?;
            let value_escaped = captures
                .get(2)
                .ok_or_else(|| ParseLogLineError::NoPropsValue)?;

            let key = UNESCAPE_PROP_KEY_RE.replace_all(key_escaped, "$1")?;
            let value = UNESCAPE_KEY_RE.replace_all(value_escaped, "$1")?;

            props.push((key, value));

            rest = &rest[capture_len..];
        }

        while let Some(captures) = EXTRA_RE.captures(rest, &mut mem) {
            let capture_len = captures.get(0).unwrap().len();
            let extra = captures.get(1).ok_or_else(|| ParseLogLineError::NoExtra)?;

            let extra = CR_RE.replace_all(extra, "\r")?;
            let extra = LF_RE.replace_all(extra.as_ref(), "\n")?;
            let extra = UNESCAPE_EXTRA_RE.replace_all(extra.as_ref(), "$1")?;

            extras.push(extra.into_owned());

            rest = &rest[capture_len..];
        }

        if !rest.is_empty() {
            return Err(ParseLogLineError::RestLeft);
        }

        let log_level = *log_level
            .as_bytes()
            .get(0)
            .ok_or_else(|| ParseLogLineError::LogLevelIsEmpty)?;

        let mut serializer = Serializer::new();

        let timestamp_string = self.timestamp_string();
        let timestamp_string = serializer.create_string(timestamp_string);

        let props: Vec<_> = props
            .into_iter()
            .map(|(key, value)| {
                let args = PropArgs {
                    key: Some(serializer.create_string(key.as_ref())),
                    value: Some(serializer.create_string(value.as_ref())),
                };

                Prop::create(serializer.buffer_builder(), &args)
            })
            .collect();

        let extras: Vec<_> = extras
            .into_iter()
            .map(|extra| serializer.create_string(extra.as_ref()))
            .collect();

        let args = ParsedLogLineArgs {
            log_level,
            timestamp_string: Some(timestamp_string),
            timestamp_milliseconds: *timestamp_ms,
            timestamp_microseconds: *microseconds as u16,
            logger_key: Some(serializer.create_string(logger_key.as_ref())),
            log_key: Some(serializer.create_string(log_key.as_ref())),
            props: Some(serializer.create_vector(&props)),
            extra: Some(serializer.create_vector(&extras)),
        };

        let value = ParsedLogLine::create(serializer.buffer_builder(), &args);
        let value = serializer.finish(value);

        Ok(value.into_owned())
    }
}
