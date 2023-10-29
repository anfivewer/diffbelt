use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct NoStdErrorWrap<T: Debug>(pub T);

impl<T: Debug> Debug for NoStdErrorWrap<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T: Debug> Display for NoStdErrorWrap<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<T: Debug> Error for NoStdErrorWrap<T> {}

impl<T: Debug> From<T> for NoStdErrorWrap<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
