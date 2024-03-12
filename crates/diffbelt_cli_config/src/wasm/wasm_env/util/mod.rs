use crate::wasm::types::WasmPtrToByte;
use crate::wasm::{WasmError, WasmStoreData};
use diffbelt_util_no_std::cast::try_positive_i32_to_usize;
use std::str::from_utf8;
use wasmtime::{Memory, StoreContext};

pub struct WasmUtf8Holder<'a> {
    ctx: StoreContext<'a, WasmStoreData>,
    memory: &'a Memory,
    start: usize,
    end: usize,
}

impl WasmUtf8Holder<'_> {
    pub fn as_str(&self) -> Result<&str, WasmError> {
        let memory = self.memory.data(&self.ctx);
        let slice = memory
            .get(self.start..self.end)
            .ok_or(WasmError::BadPointer)?;

        let result = from_utf8(slice).map_err(WasmError::Utf8);

        result
    }
}

pub fn ptr_to_utf8<'a>(
    ctx: StoreContext<'a, WasmStoreData>,
    memory: &'a Memory,
    ptr: WasmPtrToByte,
    len: i32,
) -> Result<WasmUtf8Holder<'a>, WasmError> {
    let ptr = ptr.0;
    let ptr = try_positive_i32_to_usize(ptr)
        .ok_or_else(|| WasmError::Unspecified(format!("ptr_to_utf8 got ptr {ptr}")))?;
    let len = try_positive_i32_to_usize(len)
        .ok_or_else(|| WasmError::Unspecified(format!("ptr_to_utf8 got len {len}")))?;

    Ok(WasmUtf8Holder {
        ctx,
        memory,
        start: ptr,
        end: ptr + len,
    })
}
