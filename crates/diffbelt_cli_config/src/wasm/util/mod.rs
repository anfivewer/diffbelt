use crate::wasm::{WasmError, WasmModuleInstance};
use either::Either;
use std::ops::Deref;

impl WasmModuleInstance {
    pub fn enter_memory_observe_context<T, E, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        let store = self
            .store
            .try_borrow()
            .map_err(|err| Either::Right(err.into()))?;
        let store = store.deref();
        let memory = self.allocation.memory.data(store);

        fun(memory).map_err(Either::Left)
    }
}
