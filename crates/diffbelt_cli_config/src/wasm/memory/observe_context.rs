use crate::wasm::{WasmError, WasmModuleInstance};
use either::Either;
use std::ops::Deref;
use wasmer::{MemoryView, WasmPtr};

pub struct WasmMemoryObserver<'a> {
    view: MemoryView<'a>,
}

impl WasmMemoryObserver<'_> {
    pub fn observe_byte_slice<T, E, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        ptr: WasmPtr<u8>,
        len: u32,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        let slice = ptr
            .slice(&self.view, len)
            .map_err(|err| Either::Right(err.into()))?;

        let slice = slice.access().map_err(|err| Either::Right(err.into()))?;
        let slice = slice.as_ref();

        fun(slice).map_err(Either::Left)
    }
}

impl WasmModuleInstance {
    pub fn enter_memory_observe_context<T, E, F: FnOnce(&'_ WasmMemoryObserver<'_>) -> Result<T, E>>(
        &self,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        let store = self
            .store
            .try_borrow()
            .map_err(|err| Either::Right(err.into()))?;
        let store = store.deref();

        let view = self.allocation.memory.view(store);

        let context = WasmMemoryObserver { view };

        fun(&context).map_err(Either::Left)
    }
}
