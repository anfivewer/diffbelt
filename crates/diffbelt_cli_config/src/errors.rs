#[derive(Debug)]
pub enum ConfigParsingError {
    ExpectedMap(ExpectedError),
    ExpectedSeq(ExpectedError),
    ExpectedString(ExpectedError),
    ExpectedBool(ExpectedError),
    UnknownKey(ExpectedError),
    Custom(ExpectedError),
}

#[derive(Debug)]
pub struct ExpectedError {
    pub message: String,
    pub position: u64,
}
