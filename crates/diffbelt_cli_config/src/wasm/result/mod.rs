use std::ops::DerefMut;

use either::Either;
use wasmer::WasmPtr;

use diffbelt_util_no_std::cast::{try_positive_i32_to_u32, try_usize_to_u32, u32_to_usize};
use diffbelt_wasm_binding::ptr::bytes::BytesVecRawParts;

use crate::wasm::memory::WasmVecHolder;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmModuleInstance, WasmPtrImpl};

pub struct WasmBytesSliceResult<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtr<u8>,
    pub len: u32,
}

pub struct WasmBytesSliceOwnedUnsafe {
    pub ptr: WasmPtr<u8>,
    pub len: u32,
}

impl<'a> WasmBytesSliceResult<'a> {
    pub fn view_to_vec_holder(
        instance: &'a WasmModuleInstance,
        holder: &WasmVecHolder,
    ) -> Result<Self, WasmError> {
        let store = instance.store.try_borrow()?;
        let view = instance.allocation.memory.view(&store);

        let raw_parts = holder.ptr.access(&view)?;
        let raw_parts = raw_parts.as_ref();

        let BytesVecRawParts::<WasmPtrImpl> { ptr, len, .. } = raw_parts.0;

        let len = try_positive_i32_to_u32(len)
            .ok_or_else(|| WasmError::Unspecified(format!("view_to_vec_holder: len {len}")))?;

        Ok(Self {
            instance,
            ptr: ptr.into(),
            len,
        })
    }

    pub fn bytes_offset_to_ptr(&self, offset: usize) -> Result<WasmPtr<u8>, WasmError> {
        let offset_u32 = try_usize_to_u32(offset).ok_or_else(|| {
            WasmError::Unspecified(format!("bytes_offset_to_ptr: offset too big {offset}"))
        })?;

        if offset >= u32_to_usize(self.len) {
            return Err(WasmError::Unspecified(format!(
                "bytes_offset_to_ptr: offset {offset}, but len is {}",
                self.len
            )));
        }

        let ptr = self.ptr.add_offset(offset_u32)?;

        Ok(ptr)
    }

    pub fn observe_bytes<T, E: From<WasmError>, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        self.instance.enter_memory_observe_context(|observer| {
            let slice = observer.slice_view(self.ptr, self.len)?;

            fun(slice.as_ref())
        })
    }

    pub fn into_owned_unsafe(self) -> WasmBytesSliceOwnedUnsafe {
        WasmBytesSliceOwnedUnsafe {
            ptr: self.ptr,
            len: self.len,
        }
    }
}
