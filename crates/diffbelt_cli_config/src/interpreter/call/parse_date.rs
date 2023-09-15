use crate::interpreter::call::FunctionExecution;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::statement::parse_date::ParseDateToMsStatement;
use crate::interpreter::var::Var;
use chrono::{NaiveDate, NaiveTime};
use regex::Regex;

lazy_static::lazy_static! {
    static ref DATE_1: Regex = Regex::new(
        "^(\\d{4})-(\\d\\d)-(\\d\\d)T(\\d\\d):(\\d\\d):(\\d\\d)\\.(\\d{1,3})Z$"
    ).unwrap();
}

impl<'a> FunctionExecution<'a> {
    pub fn execute_parse_date_to_ms(
        &mut self,
        statement: &ParseDateToMsStatement,
    ) -> Result<(), InterpreterError> {
        let ParseDateToMsStatement { ptr, mark } = statement;

        let input = self.read_var_as_str(ptr, None)?;

        let captures = DATE_1.captures(input).ok_or_else(|| {
            InterpreterError::custom_with_mark("No date matched".to_string(), mark.clone())
        })?;

        let mut result = [0; 7];

        for i in 0..7 {
            let m = captures.get(i + 1).ok_or_else(|| {
                InterpreterError::custom_with_mark(
                    "Impossible: parse_date_to_ms: empty group".to_string(),
                    mark.clone(),
                )
            })?;

            let s = m.as_str();

            let number = str::parse::<i32>(s).map_err(|_| {
                InterpreterError::custom_with_mark(
                    format!("Impossible: parse_date_to_ms: not a number \"{}\"", s),
                    mark.clone(),
                )
            })?;

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
            InterpreterError::custom_with_mark(
                "parse_date_to_ms: invalid time".to_string(),
                mark.clone(),
            )
        })?;

        let date = NaiveDate::from_ymd_opt(year, month as u32, day as u32).ok_or_else(|| {
            InterpreterError::custom_with_mark(
                "parse_date_to_ms: invalid date".to_string(),
                mark.clone(),
            )
        })?;

        let date_time = date.and_time(time).and_utc();

        let timestamp = date_time.timestamp_millis() as u64;

        self.set_var(ptr, Var::new_u64(timestamp))?;

        self.statement_index += 1;

        Ok(())
    }
}
