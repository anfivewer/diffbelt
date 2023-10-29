#[repr(i32)]
pub enum ErrorCode {
    Ok = 0,
    Fail = 1,
}

impl ErrorCode {
    pub fn from_repr(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            _ => Self::Fail,
        }
    }
}
