use crate::wasm::types::{WasmBytesVecFull, WasmPtrImpl};
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{export_error_context, WasmError, WasmModuleInstance};
use diffbelt_util::cast::try_usize_to_i32;
use diffbelt_wasm_binding::BytesVecFull;
use std::mem;
use wasmer::{AsStoreRef, Instance, Memory, TypedFunction, WasmPtr};

#[derive(Clone)]
pub struct Allocation {
    pub alloc: TypedFunction<i32, WasmPtr<u8>>,
    pub dealloc: TypedFunction<(WasmPtr<u8>, i32), ()>,
    pub alloc_bytes_vec_full_struct: TypedFunction<(), WasmPtr<WasmBytesVecFull>>,
    pub dealloc_bytes_vec_full_struct: TypedFunction<WasmPtr<WasmBytesVecFull>, ()>,
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
            .get_typed_function(&store, "alloc_bytes_vec_full_struct")
            .map_err(export_error_context(|| {
                "alloc_bytes_vec_full_struct()".to_string()
            }))?;
        let dealloc_bytes_vec_full_struct = instance
            .exports
            .get_typed_function(&store, "dealloc_bytes_vec_full_struct")
            .map_err(export_error_context(|| {
                "dealloc_bytes_vec_full_struct()".to_string()
            }))?;

        Ok(Self {
            alloc,
            dealloc,
            alloc_bytes_vec_full_struct,
            dealloc_bytes_vec_full_struct,
            memory,
        })
    }
}

pub struct WasmVecHolder {
    pub holder: WasmPtr<BytesVecFull<WasmPtrImpl>>,
}

impl WasmEnv {
    pub fn set_memory(&self, memory: Memory) {
        let mut lock = self.memory.lock().unwrap();
        lock.replace(memory);
    }

    pub fn set_allocation(&self, allocation: Allocation) {
        let mut lock = self.allocation.lock().unwrap();
        lock.replace(allocation);
    }
}

impl WasmModuleInstance {
    pub fn alloc_vec_holder(&self) -> Result<WasmVecHolder, WasmError> {
        let mut store = self.store.try_borrow_mut()?;

        let ptr = self.allocation.alloc.call(
            &mut store,
            try_usize_to_i32(mem::size_of::<BytesVecFull<WasmPtrImpl>>()).ok_or_else(|| {
                WasmError::Unspecified(format!("alloc_vec_and_full_holder: BytesVecFull alloc"))
            })?,
        )?;

        Ok(WasmVecHolder { holder: ptr.cast() })
    }
}
