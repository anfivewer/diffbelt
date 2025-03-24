use crate::error_code::ErrorCode;
use crate::ptr::{ConstPtr, MutPtr};
use crate::requests::RequestId;
use diffbelt_util_no_std::cast::try_usize_to_u32;

mod extern_functions {
    use crate::error_code::ErrorCode;
    use crate::ptr::ConstPtr;

    #[link(wasm_import_module = "Diffbelt")]
    unsafe extern "C" {
        /// Receives name of transform, runs it until fully finished
        pub fn run_transform(slice_ptr: ConstPtr<u8>, slice_len: u32) -> ErrorCode;
        pub fn sleep_ms(duration_ms: u32) -> ErrorCode;
    }
}

pub fn run_transform(name: &str) -> Result<(), ErrorCode> {
    let ptr = ConstPtr::from(name.as_ptr());
    let len = try_usize_to_u32(name.as_bytes().len()).ok_or_else(|| ErrorCode::SafeFail)?;

    // SAFETY: trust in host
    let code = unsafe { extern_functions::run_transform(ptr, len) };

    code.as_result()
}

pub fn sleep_ms(duration_ms: u32) -> Result<(), ErrorCode> {
    // SAFETY: trust in host
    let code = unsafe { extern_functions::sleep_ms(duration_ms) };
    code.as_result()
}
