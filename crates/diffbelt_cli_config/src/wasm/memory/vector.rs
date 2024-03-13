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

pub struct WasmVecHolder<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtrToVecRawParts,
}

impl WasmModuleInstance {
    pub fn alloc_vec_holder(&self) -> Result<WasmVecHolder<'_>, WasmError> {
        let mut store = self.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let ptr = self.allocation.alloc_bytes_vec_raw_parts.call(store, ())?;

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

        let len = try_positive_i32_to_usize(raw_parts.len)
            .ok_or_else(|| WasmError::Unspecified(format!("access_vec: len {}", raw_parts.len)))?;

        let result = WasmBytesSliceResult {
            instance: self.instance,
            ptr: raw_parts.ptr,
            len,
        };

        Ok(result)
    }

    pub fn replace_with_slice_and_return_slice(
        &self,
        slice: &[u8],
    ) -> Result<WasmBytesSlice, WasmError> {
        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let len = try_usize_to_i32(slice.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("replace_vec_with_slice: slice len {}", slice.len()))
        })?;
        let len_u32 = unchecked_i32_to_u32(len);

        () = self
            .instance
            .allocation
            .ensure_vec_capacity
            .call(store, (self.ptr, len))?;

        let memory = self.instance.allocation.memory.data_mut(store);
        let raw_parts = self.ptr.as_mut(memory)?;
        raw_parts.0.len = len;

        let vec_ptr = raw_parts.0.ptr;

        let vec_slice = vec_ptr.slice()?;
        () = vec_slice.write_slice(memory, slice)?;

        let wasm_slice = WasmBytesSlice(SliceRawParts { ptr: vec_ptr, len });

        Ok(wasm_slice)
    }

    pub fn replace_with_slice(&self, slice: &[u8]) -> Result<(), WasmError> {
        _ = self.replace_with_slice_and_return_slice(slice)?;

        Ok(())
    }
}

impl Drop for WasmVecHolder<'_> {
    fn drop(&mut self) {
        let result = (|| {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            () = self
                .instance
                .allocation
                .dealloc_bytes_vec_raw_parts
                .call(store, self.ptr)?;

            Ok::<(), WasmError>(())
        })();

        () = WasmEnv::handle_error(&self.instance.error, result).unwrap_or(());
    }
}
