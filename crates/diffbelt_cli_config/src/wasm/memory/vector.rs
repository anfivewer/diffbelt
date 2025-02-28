use std::ops::{Deref, DerefMut};

use either::Either;
use wasmtime::AsContextMut;

use diffbelt_util_no_std::cast::{try_usize_to_u32, u32_to_usize};
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use diffbelt_wasm_binding::ptr::slice::SliceRawParts;

use crate::wasm::memory::DeallocType;
use crate::wasm::result::WasmBytesSliceResult;
use crate::wasm::types::{WasmBytesSlice, WasmPtrToVecRawParts};
use crate::wasm::WasmModuleInstance;
use crate::wasm::error::WasmError;

// FIXME: we cannot use any WasmVecHolder which was passed to wasm if function paniced/failed,
//        we should mark them as broken and not try to dealloc and replace it with fresh one.
//        Maybe there should be also ExitCode::SafeFail, which user provides, if vectors are stored
//        back in a consistent state
pub struct WasmVecHolder<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtrToVecRawParts,
}

impl WasmModuleInstance {
    pub async fn alloc_vec_holder(&self) -> Result<WasmVecHolder<'_>, WasmError> {
        let mut store = self.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let ptr = self
            .allocation
            .alloc_bytes_vec_raw_parts
            .call_async(store, ())
            .await?;

        Ok(WasmVecHolder {
            instance: self,
            ptr,
        })
    }

    pub fn vec_to_bytes_slice(
        &self,
        vec_holder: &WasmVecHolder<'_>,
    ) -> Result<WasmBytesSlice, WasmError> {
        let store = self.store.try_borrow()?;
        let store = store.deref();

        let memory = self.allocation.memory.data(store);
        let raw_parts = vec_holder.ptr.access(memory)?;
        let raw_parts = raw_parts.0;

        let slice = WasmBytesSlice(BytesSlice {
            ptr: raw_parts.ptr,
            len: raw_parts.len,
        });

        Ok(slice)
    }
}

impl<'a> WasmVecHolder<'a> {
    pub fn read_slice(&self) -> Result<WasmBytesSlice, WasmError> {
        let store = self.instance.store.try_borrow()?;
        let store = store.deref();

        let memory = self.instance.allocation.memory.data(store);
        let raw_parts = self.ptr.access(memory)?;

        Ok(WasmBytesSlice(BytesSlice {
            ptr: raw_parts.0.ptr,
            len: raw_parts.0.len,
        }))
    }

    pub fn observe_slice<T, E: From<WasmError>, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        instance: &WasmModuleInstance,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        instance.enter_memory_observe_context(|memory| {
            let raw_parts = self.ptr.access(memory)?;

            let ptr = u32_to_usize(raw_parts.0.ptr.value);
            let len = u32_to_usize(raw_parts.0.len);

            fun(&memory[ptr..(ptr + len)])
        })
    }

    #[deprecated(note = "use read_slice()/observe_slice()")]
    pub fn access(&self) -> Result<WasmBytesSliceResult<'a>, WasmError> {
        let store = self.instance.store.try_borrow()?;
        let store = store.deref();

        let memory = self.instance.allocation.memory.data(store);
        let raw_parts = self.ptr.access(memory)?;
        let raw_parts = raw_parts.0;
        let raw_parts_len = raw_parts.len;

        let len = u32_to_usize(raw_parts_len);

        let result = WasmBytesSliceResult {
            instance: self.instance,
            ptr: raw_parts.ptr,
            len,
        };

        Ok(result)
    }

    pub async fn replace_with_slice_and_return_slice(
        &self,
        slice: &[u8],
    ) -> Result<WasmBytesSlice, WasmError> {
        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let len = try_usize_to_u32(slice.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("replace_vec_with_slice: slice len {}", slice.len()))
        })?;

        let () = self
            .instance
            .allocation
            .ensure_vec_capacity
            .call_async(store.as_context_mut(), (self.ptr, len))
            .await?;

        let memory = self
            .instance
            .allocation
            .memory
            .data_mut(store.as_context_mut());
        let raw_parts = self.ptr.as_mut(memory)?;
        raw_parts.0.len = len;

        let vec_ptr = raw_parts.0.ptr;

        let vec_slice = vec_ptr.slice();
        let () = vec_slice.write_slice(memory, slice)?;

        let wasm_slice = WasmBytesSlice(SliceRawParts { ptr: vec_ptr, len });

        Ok(wasm_slice)
    }

    pub async fn replace_with_slice(&self, slice: &[u8]) -> Result<(), WasmError> {
        _ = self.replace_with_slice_and_return_slice(slice).await?;

        Ok(())
    }
}

impl<'a> AsRef<WasmVecHolder<'a>> for WasmVecHolder<'a> {
    fn as_ref(&self) -> &WasmVecHolder<'a> {
        self
    }
}

impl Drop for WasmVecHolder<'_> {
    fn drop(&mut self) {
        let mut pending_deallocs = self
            .instance
            .allocation
            .pending_deallocs
            .lock()
            .expect("lock");
        pending_deallocs.push(DeallocType::VecHolder { ptr: self.ptr });
    }
}
