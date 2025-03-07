#[derive(Eq, PartialEq, Copy, Clone, Debug)]
#[repr(i32)]
pub enum ErrorCode {
    Ok = 0,
    SafeFail = 1,
    UnsafeFail = 2,
    LimitReached = 3,
}

impl ErrorCode {
    pub fn repr(self) -> i32 {
        self as i32
    }

    pub fn from_repr(value: i32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::SafeFail,
            _ => Self::UnsafeFail,
        }
    }

    pub fn is_error(self) -> bool {
        match self {
            ErrorCode::Ok => false,
            _ => true,
        }
    }
}
