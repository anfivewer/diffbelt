use crate::wasm::types::{WasmBytesVecRawParts, WasmPtrImpl};
use crate::wasm::{export_error_context, WasmError, WasmModuleInstance};
use diffbelt_util::cast::try_usize_to_i32;
use diffbelt_wasm_binding::bytes::BytesVecRawParts;
use std::mem;
use std::ops::DerefMut;
use wasmer::{AsStoreRef, Instance, Memory, TypedFunction, WasmPtr};
use crate::wasm::result::WasmBytesSliceResult;
use crate::wasm::wasm_env::WasmEnv;

#[derive(Clone)]
pub struct Allocation {
    pub alloc: TypedFunction<i32, WasmPtr<u8>>,
    pub dealloc: TypedFunction<(WasmPtr<u8>, i32), ()>,
    pub alloc_bytes_vec_raw_parts: TypedFunction<(), WasmPtr<WasmBytesVecRawParts>>,
    pub dealloc_bytes_vec_raw_parts: TypedFunction<WasmPtr<WasmBytesVecRawParts>, ()>,
    pub memory: Memory,
}

impl Allocation {
    pub fn new(
        store: &(impl AsStoreRef + ?Sized),
        instance: &Instance,
        memory: Memory,
    ) -> Result<Self, WasmError> {
        let alloc = instance
            .exports
            .get_typed_function(&store, "alloc")
            .map_err(export_error_context(|| "alloc()".to_string()))?;
        let dealloc = instance
            .exports
            .get_typed_function(&store, "dealloc")
            .map_err(export_error_context(|| "dealloc()".to_string()))?;
        let alloc_bytes_vec_full_struct = instance
            .exports
            .get_typed_function(&store, "alloc_bytes_vec_raw_parts")
            .map_err(export_error_context(|| {
                "alloc_bytes_vec_raw_parts()".to_string()
            }))?;
        let dealloc_bytes_vec_full_struct = instance
            .exports
            .get_typed_function(&store, "dealloc_bytes_vec_raw_parts")
            .map_err(export_error_context(|| {
                "dealloc_bytes_vec_raw_parts()".to_string()
            }))?;

        Ok(Self {
            alloc,
            dealloc,
            alloc_bytes_vec_raw_parts: alloc_bytes_vec_full_struct,
            dealloc_bytes_vec_raw_parts: dealloc_bytes_vec_full_struct,
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

    pub fn access_slice(&self) -> Result<WasmBytesSliceResult, WasmError> {
        //

        todo!()
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