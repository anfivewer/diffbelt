use either::Either;
use std::ops::DerefMut;

use crate::wasm::types::{WasmBytesSlice, WasmPtr, WasmPtrToBytesSlice};
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmModuleInstance};

pub struct WasmSliceHolder<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtr<WasmBytesSlice>,
}

impl WasmModuleInstance {
    pub fn alloc_slice_holder(&self) -> Result<WasmSliceHolder<'_>, WasmError> {
        let mut store = self.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let ptr = self.allocation.alloc_bytes_slice.call(store, ())?;

        Ok(WasmSliceHolder {
            instance: self,
            ptr,
        })
    }
}

impl WasmBytesSlice {
    pub fn observe_bytes<T, E: From<WasmError>, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        instance: &WasmModuleInstance,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        instance.enter_memory_observe_context(|memory| {
            let slice = self.access(memory)?;

            fun(slice)
        })
    }
}

impl Drop for WasmSliceHolder<'_> {
    fn drop(&mut self) {
        let result = (|| {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            () = self
                .instance
                .allocation
                .dealloc_bytes_slice
                .call(store, self.ptr)?;

            Ok::<(), WasmError>(())
        })();

        () = WasmEnv::handle_error(&self.instance.error, result).unwrap_or(());
    }
}
