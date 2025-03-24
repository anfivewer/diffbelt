use crate::error_code::ErrorCode;
use crate::ptr::ConstPtr;
use crate::schedule_dealloc::ScheduleDealloc;
use alloc::string::String;
use diffbelt_util_no_std::cast::checked_usize_to_u32;

pub trait IntegrationTest {
    unsafe extern "C" fn test() -> ErrorCode;
}

#[link(wasm_import_module = "Diffbelt")]
unsafe extern "C" {
    /// Should be slice to a string. Backing String can be freed by [`crate::allocator::schedule_bytes_vec_dealloc`]
    fn set_test_error(slice_ptr: ConstPtr<u8>, slice_len: u32);
}

/// Marks current test as failed, replaces error message with `error`
pub fn report_single_test_error(error: String) {
    let s = error.as_bytes();
    let s_ptr = ConstPtr::from(s.as_ptr());
    let s_len = checked_usize_to_u32(s.len());
    error.schedule_dealloc();
    // SAFETY: trust in host, string is consumed
    unsafe {
        set_test_error(s_ptr, s_len);
    }
}
