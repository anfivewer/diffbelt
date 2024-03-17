use either::Either;
use std::ops::Deref;

use diffbelt_util_no_std::cast::{try_positive_i32_to_usize, try_usize_to_i32};
use diffbelt_wasm_binding::ptr::bytes::BytesVecRawParts;

use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::WasmPtr;
use crate::wasm::{WasmError, WasmModuleInstance, WasmPtrImpl};

#[deprecated(note = "Use just WasmBytesSlice")]
pub struct WasmBytesSliceResult<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtr<u8>,
    pub len: usize,
}

pub struct WasmBytesSliceOwnedUnsafe {
    pub ptr: WasmPtr<u8>,
    pub len: usize,
}

impl<'a> WasmBytesSliceResult<'a> {
    pub fn view_to_vec_holder(
        instance: &'a WasmModuleInstance,
        holder: &WasmVecHolder,
    ) -> Result<Self, WasmError> {
        let store = instance.store.try_borrow()?;
        let store = store.deref();
        let memory = instance.allocation.memory.data(store);

        let raw_parts = holder.ptr.access(memory)?;

        let BytesVecRawParts::<WasmPtrImpl> { ptr, len, .. } = raw_parts.0;

        let len = try_positive_i32_to_usize(len)
            .ok_or_else(|| WasmError::Unspecified(format!("view_to_vec_holder: len {len}")))?;

        Ok(Self {
            instance,
            ptr: ptr.into(),
            len,
        })
    }

    pub fn bytes_offset_to_ptr(&self, offset: usize) -> Result<WasmPtr<u8>, WasmError> {
        let offset_i32 = try_usize_to_i32(offset).ok_or_else(|| {
            WasmError::Unspecified(format!("bytes_offset_to_ptr: offset too big {offset}"))
        })?;

        if offset >= self.len {
            return Err(WasmError::Unspecified(format!(
                "bytes_offset_to_ptr: offset {offset}, but len is {}",
                self.len
            )));
        }

        let ptr = self.ptr.add_offset(offset_i32)?;

        Ok(ptr)
    }

    #[deprecated(note = "Use just WasmBytesSlice")]
    pub fn observe_bytes<T, E: From<WasmError>, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        self.instance.enter_memory_observe_context(|memory| {
            let slice = self.ptr.slice()?.slice(memory, self.len)?;

            fun(slice)
        })
    }

    pub fn into_owned_unsafe(self) -> WasmBytesSliceOwnedUnsafe {
        WasmBytesSliceOwnedUnsafe {
            ptr: self.ptr,
            len: self.len,
        }
    }
}
