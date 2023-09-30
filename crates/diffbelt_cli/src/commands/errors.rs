use std::str::Utf8Error;
use diffbelt_cli_config::config_tests::run::RunTestsError;

#[derive(Debug)]
pub enum CommandError {
    Unknown,
    Message(String),
    Io(std::io::Error),
    Utf8(Utf8Error),
    RunTests(RunTestsError),
}
