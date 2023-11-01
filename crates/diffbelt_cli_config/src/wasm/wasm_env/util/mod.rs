use crate::wasm::WasmError;
use diffbelt_util_no_std::cast::try_positive_i32_to_u32;
use std::str::from_utf8;
use wasmer::{MemoryView, WasmPtr, WasmSliceAccess};

pub struct WasmUtf8Holder<'a> {
    slice: WasmSliceAccess<'a, u8>,
}

impl WasmUtf8Holder<'_> {
    pub fn as_str(&self) -> Result<&str, WasmError> {
        let slice = self.slice.as_ref();

        let result = from_utf8(slice).map_err(WasmError::Utf8);

        result
    }
}

pub fn ptr_to_utf8<'a>(
    view: &'a MemoryView,
    ptr: WasmPtr<u8>,
    len: i32,
) -> Result<WasmUtf8Holder<'a>, WasmError> {
    let len = try_positive_i32_to_u32(len)
        .ok_or_else(|| WasmError::Unspecified(format!("ptr_to_utf8 got len {len}")))?;

    let slice = ptr.slice(view, len).map_err(WasmError::MemoryAccess)?;
    let slice = slice.access().map_err(WasmError::MemoryAccess)?;

    Ok(WasmUtf8Holder { slice })
}
