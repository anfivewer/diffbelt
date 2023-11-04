use std::ops::DerefMut;

use wasmer::{AsStoreRef, Instance, Memory, TypedFunction, WasmPtr};

use diffbelt_util_no_std::cast::{try_positive_i32_to_u32, try_usize_to_i32, unchecked_i32_to_u32};
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;

use crate::wasm::result::WasmBytesSliceResult;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts};
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{export_error_context, WasmError, WasmModuleInstance};

pub mod observe_context;

#[derive(Clone)]
pub struct Allocation {
    pub alloc: TypedFunction<i32, WasmPtr<u8>>,
    pub dealloc: TypedFunction<(WasmPtr<u8>, i32), ()>,
    pub alloc_bytes_slice: TypedFunction<(), WasmPtr<WasmBytesSlice>>,
    pub dealloc_bytes_slice: TypedFunction<WasmPtr<WasmBytesSlice>, ()>,
    pub alloc_bytes_vec_raw_parts: TypedFunction<(), WasmPtr<WasmBytesVecRawParts>>,
    pub dealloc_bytes_vec_raw_parts: TypedFunction<WasmPtr<WasmBytesVecRawParts>, ()>,
    pub ensure_vec_capacity: TypedFunction<(WasmPtr<WasmBytesVecRawParts>, i32), ()>,
    pub memory: Memory,
}

impl Allocation {
    pub fn new(
        store: &(impl AsStoreRef + ?Sized),
        instance: &Instance,
        memory: Memory,
    ) -> Result<Self, WasmError> {
        macro_rules! get_function {
            ($name:ident, $name_text:literal) => {
                let $name = instance
                    .exports
                    .get_typed_function(&store, $name_text)
                    .map_err(export_error_context(|| {
                        concat!($name_text, "()").to_string()
                    }))?;
            };
        }

        get_function!(alloc, "alloc");
        get_function!(dealloc, "dealloc");
        get_function!(alloc_bytes_slice, "alloc_bytes_slice");
        get_function!(dealloc_bytes_slice, "dealloc_bytes_slice");
        get_function!(alloc_bytes_vec_raw_parts, "alloc_bytes_vec_raw_parts");
        get_function!(dealloc_bytes_vec_raw_parts, "dealloc_bytes_vec_raw_parts");
        get_function!(ensure_vec_capacity, "ensure_vec_capacity");

        Ok(Self {
            alloc,
            dealloc,
            alloc_bytes_slice,
            dealloc_bytes_slice,
            alloc_bytes_vec_raw_parts,
            dealloc_bytes_vec_raw_parts,
            ensure_vec_capacity,
            memory,
        })
    }
}

pub struct WasmVecHolder<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtr<WasmBytesVecRawParts>,
}

impl WasmModuleInstance {
    pub fn alloc_vec_holder(&self) -> Result<WasmVecHolder<'_>, WasmError> {
        let mut store = self.store.try_borrow_mut()?;

        let ptr = self.allocation.alloc_bytes_vec_raw_parts.call(&mut store)?;

        Ok(WasmVecHolder {
            instance: self,
            ptr,
        })
    }

    pub fn replace_vec_with_slice(
        &self,
        vec_holder: &WasmVecHolder<'_>,
        slice: &[u8],
    ) -> Result<(), WasmError> {
        let mut store = self.store.try_borrow_mut()?;

        let len = try_usize_to_i32(slice.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("replace_vec_with_slice: slice len {}", slice.len()))
        })?;
        let len_u32 = unchecked_i32_to_u32(len);

        () = self
            .allocation
            .ensure_vec_capacity
            .call(&mut store, vec_holder.ptr, len)?;

        let view = self.allocation.memory.view(&store);
        let mut access = vec_holder.ptr.access(&view)?;

        {
            let raw_parts = access.as_mut();
            raw_parts.0.len = len;
        }

        let vec_ptr = WasmPtr::from(access.as_ref().0.ptr);

        let vec_slice = vec_ptr.slice(&view, len_u32)?;
        () = vec_slice.write_slice(slice)?;

        Ok(())
    }

    pub fn vec_to_bytes_slice(
        &self,
        vec_holder: &WasmVecHolder<'_>,
    ) -> Result<WasmBytesSlice, WasmError> {
        let store = self.store.try_borrow()?;

        let view = self.allocation.memory.view(&store);
        let access = vec_holder.ptr.access(&view)?;

        let raw_parts = access.as_ref();

        let slice = WasmBytesSlice(BytesSlice {
            ptr: raw_parts.0.ptr,
            len: raw_parts.0.len,
        });

        Ok(slice)
    }

    pub fn access_vec<'a>(
        &'a self,
        vec_holder: &WasmVecHolder<'a>,
    ) -> Result<WasmBytesSliceResult<'a>, WasmError> {
        let store = self.store.try_borrow()?;

        let view = self.allocation.memory.view(&store);
        let access = vec_holder.ptr.access(&view)?;

        let raw_parts = access.as_ref();

        let len = try_positive_i32_to_u32(raw_parts.0.len).ok_or_else(|| {
            WasmError::Unspecified(format!("access_vec: len {}", raw_parts.0.len))
        })?;

        let result = WasmBytesSliceResult {
            instance: self,
            ptr: raw_parts.0.ptr.into(),
            len,
        };

        Ok(result)
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
