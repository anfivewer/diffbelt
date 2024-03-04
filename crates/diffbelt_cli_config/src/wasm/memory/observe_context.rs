use std::ops::Deref;

use either::Either;
use wasmer::{MemoryView, WasmPtr, WasmSliceAccess};
use wasmer_types::ValueType;

use diffbelt_util_no_std::cast::try_positive_i32_to_u32;

use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::WasmBytesSlice;
use crate::wasm::{WasmError, WasmModuleInstance};

pub struct WasmMemoryObserver<'a> {
    view: MemoryView<'a>,
}

pub struct WasmSliceView<'a, T: ValueType> {
    slice: WasmSliceAccess<'a, T>,
}

impl<T: ValueType> WasmSliceView<'_, T> {
    pub fn as_ref(&self) -> &[T] {
        self.slice.as_ref()
    }
}

impl WasmMemoryObserver<'_> {
    pub fn slice_view<T: ValueType>(
        &self,
        ptr: WasmPtr<T>,
        len: u32,
    ) -> Result<WasmSliceView<'_, T>, WasmError> {
        let slice = ptr.slice(&self.view, len)?;
        let slice = slice.access()?;

        Ok(WasmSliceView { slice })
    }

    pub fn bytes_slice_slice_view(
        &self,
        slice: WasmPtr<WasmBytesSlice>,
    ) -> Result<WasmSliceView<'_, u8>, WasmError> {
        let slice = slice.read(&self.view)?;
        let ptr = WasmPtr::from(slice.0.ptr);
        let len = try_positive_i32_to_u32(slice.0.len).ok_or_else(|| {
            WasmError::Unspecified(format!("bytes_slice_slice_view len {}", slice.0.len))
        })?;
        let slice = ptr.slice(&self.view, len)?;
        let slice = slice.access()?;

        Ok(WasmSliceView { slice })
    }

    pub fn vec_view(&self, holder: &WasmVecHolder) -> Result<WasmSliceView<'_, u8>, WasmError> {
        let holder = holder.ptr.access(&self.view)?;
        let holder = holder.as_ref();

        let ptr = WasmPtr::from(holder.0.ptr);
        let len = try_positive_i32_to_u32(holder.0.len).ok_or_else(|| {
            WasmError::Unspecified(format!("WasmMemoryObserver::vec_view len {}", holder.0.len))
        })?;

        self.slice_view(ptr, len)
    }
}

impl WasmModuleInstance {
    pub fn enter_memory_observe_context<
        T,
        E,
        F: FnOnce(&'_ WasmMemoryObserver<'_>) -> Result<T, E>,
    >(
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
