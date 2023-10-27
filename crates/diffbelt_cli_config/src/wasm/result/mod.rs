use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmModuleInstance};
use either::Either;
use std::cell::RefMut;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use wasmer::{MemoryView, Store, WasmPtr};

pub struct WasmBytesSliceResult<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtr<u8>,
    pub len: u32,

    pub on_drop_dealloc: Option<(WasmPtr<u8>, i32)>,
}

impl WasmBytesSliceResult<'_> {
    pub fn observe_bytes<T, E, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        let mut store = self
            .instance
            .store
            .try_borrow_mut()
            .map_err(|err| Either::Right(err.into()))?;
        let store = store.deref_mut();

        let view = self.instance.allocation.memory.view(store);

        let slice = self
            .ptr
            .slice(&view, self.len)
            .map_err(|err| Either::Right(err.into()))?;

        let slice = slice.access().map_err(|err| Either::Right(err.into()))?;
        let slice = slice.as_ref();

        fun(slice).map_err(Either::Left)
    }
}

impl Drop for WasmBytesSliceResult<'_> {
    fn drop(&mut self) {
        let Some((ptr, len)) = self.on_drop_dealloc.take() else {
            return;
        };

        if len <= 0 {
            return;
        }

        let result = (|| {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            () = self
                .instance
                .allocation
                .dealloc
                .call(store, ptr.into(), len)?;

            Ok::<(), WasmError>(())
        })();

        () = WasmEnv::handle_error(&self.instance.error, result).unwrap_or(());
    }
}
