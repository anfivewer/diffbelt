use crate::util::cast::try_positive_i64_to_u64;
use alloc::format;
use alloc::string::{String, ToString};
use chrono::{NaiveDate, NaiveTime};
use core::num::ParseIntError;
use diffbelt_wasm_binding::Regex;
use thiserror_no_std::Error;

lazy_static::lazy_static! {
    static ref DATE_RE: Regex = Regex::new(
        "^(\\d{4})-(\\d\\d)-(\\d\\d)T(\\d\\d):(\\d\\d):(\\d\\d)\\.(\\d{1,3})Z$"
    ).expect("Cannot build DATE_RE");
}

#[derive(Error, Debug)]
pub enum ParseDateError {
    ParseInt(#[from] ParseIntError),
    NotMatchesRegexp(String),
    NoCapture { i: usize },
    NaiveTime(String),
    NaiveDate(String),
    Unspecified,
}

pub fn parse_date_to_timestamp_ms(input: &str) -> Result<u64, ParseDateError> {
    let mut mem = Regex::alloc_captures::<8>();
    let captures = DATE_RE
        .captures(input, &mut mem)
        .ok_or_else(|| ParseDateError::NotMatchesRegexp(input.to_string()))?;

    let mut result = [0; 7];

    for i in 0..7 {
        let s = captures
            .get(i + 1)
            .ok_or_else(|| ParseDateError::NoCapture { i: i + 1 })?;

        let number = str::parse::<i32>(s)?;

        result[i] = number;
    }

    let [year, month, day, hours, minutes, seconds, milliseconds] = result;

    let time = NaiveTime::from_hms_milli_opt(
        hours as u32,
        minutes as u32,
        seconds as u32,
        milliseconds as u32,
    )
    .ok_or_else(|| {
        ParseDateError::NaiveTime(format!("{hours}:{minutes}:{seconds}:{milliseconds}"))
    })?;

    let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32)
        .ok_or_else(|| ParseDateError::NaiveDate(format!("{year}-{month}-{day}")))?;

    let date_time = date.and_time(time).and_utc();

    let timestamp =
        try_positive_i64_to_u64(date_time.timestamp_millis()).ok_or(ParseDateError::Unspecified)?;

    Ok(timestamp)
}
