#[derive(Eq, PartialEq)]
#[derive(Debug)]
#[repr(i32)]
pub enum ErrorCode {
    Ok = 0,
    SafeFail = 1,
    UnsafeFail = 2,
}

impl ErrorCode {
    pub fn from_repr(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::SafeFail,
            _ => Self::UnsafeFail,
        }
    }
}
