use crate::wasm::memory::DeallocType;
use crate::wasm::result::WasmBytesSliceResult;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtrToVecRawParts};
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmModuleInstance};
use diffbelt_util_no_std::cast::{
    try_positive_i32_to_u32, try_positive_i32_to_usize, try_usize_to_i32, unchecked_i32_to_u32,
};
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use diffbelt_wasm_binding::ptr::slice::SliceRawParts;
use std::ops::{Deref, DerefMut};
use wasmtime::AsContextMut;

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
    pub fn access(&self) -> Result<WasmBytesSliceResult<'a>, WasmError> {
        let store = self.instance.store.try_borrow()?;
        let store = store.deref();

        let memory = self.instance.allocation.memory.data(store);
        let raw_parts = self.ptr.access(memory)?;
        let raw_parts = raw_parts.0;
        let raw_parts_len = raw_parts.len;

        let len = try_positive_i32_to_usize(raw_parts_len)
            .ok_or_else(|| WasmError::Unspecified(format!("access_vec: len {}", raw_parts_len)))?;

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

        let len = try_usize_to_i32(slice.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("replace_vec_with_slice: slice len {}", slice.len()))
        })?;

        () = self
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

        let vec_slice = vec_ptr.slice()?;
        () = vec_slice.write_slice(memory, slice)?;

        let wasm_slice = WasmBytesSlice(SliceRawParts { ptr: vec_ptr, len });

        Ok(wasm_slice)
    }

    pub async fn replace_with_slice(&self, slice: &[u8]) -> Result<(), WasmError> {
        _ = self.replace_with_slice_and_return_slice(slice).await?;

        Ok(())
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
