use core::str::Utf8Error;
use crate::annotations::Annotated;
use crate::ptr::bytes::BytesSlice;

impl Annotated<BytesSlice, &str> {
    #[inline(always)]
    pub unsafe fn as_str(&self) -> Result<&str, Utf8Error> {
        self.value.as_str()
    }
}