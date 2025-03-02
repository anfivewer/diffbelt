use crate::allocator::dealloc_bytes_vec_raw_parts;
use crate::ptr::bytes::BytesVecRawParts;
use crate::ptr::{ConstPtr, MutPtr};
use alloc::string::String;
use alloc::vec::Vec;
use diffbelt_wasm_binding::allocator::alloc_bytes_vec_raw_parts;

pub trait ScheduleDealloc {
    fn schedule_dealloc(self);
}

#[link(wasm_import_module = "Diffbelt")]
extern "C" {
    /// asks executor to call [`dealloc_bytes_vec`] after current function end when it
    /// does not need any data from current function results/outputs
    fn schedule_bytes_vec_dealloc(ptr: MutPtr<BytesVecRawParts>);
}

impl ScheduleDealloc for Vec<u8> {
    fn schedule_dealloc(self) {
        let mut parts = BytesVecRawParts::from(self);
        // SAFETY: trust in host
        unsafe {
            schedule_bytes_vec_dealloc(MutPtr::from(&mut parts as *mut _));
        }
    }
}

impl ScheduleDealloc for String {
    fn schedule_dealloc(self) {
        self.into_bytes().schedule_dealloc();
    }
}
