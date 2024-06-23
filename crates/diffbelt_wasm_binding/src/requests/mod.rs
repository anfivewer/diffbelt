use crate::error_code::ErrorCode;
use crate::ptr::bytes::BytesVecRawParts;
use crate::ptr::{ConstPtr, MutPtr};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[repr(C)]
pub struct RequestId(pub u32);

impl RequestId {
    pub fn invalid() -> Self {
        Self(0)
    }

    pub fn limit_reached() -> Self {
        Self(1)
    }

    pub fn is_valid(&self) -> bool {
        self.0 > 1
    }
}

#[link(wasm_import_module = "Diffbelt")]
extern "C" {
    fn request(slice_ptr: ConstPtr<u8>, slice_len: u32) -> RequestId;
    fn is_request_finished(request_id: RequestId) -> ErrorCode;
    fn on_request_finished(
        request_id: RequestId,
        vec_ptr: MutPtr<BytesVecRawParts>,
        offset_ptr: MutPtr<u32>,
        len_ptr: MutPtr<u32>,
    ) -> ErrorCode;
}
