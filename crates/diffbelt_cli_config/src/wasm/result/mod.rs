use std::ops::{Deref, DerefMut};

use either::Either;
use wasmer::{MemoryView, WasmPtr};

use diffbelt_util::cast::{try_positive_i32_to_u32, try_usize_to_u32, u32_to_usize};
use diffbelt_wasm_binding::bytes::BytesVecRawParts;

use crate::wasm::memory::WasmVecHolder;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmModuleInstance, WasmPtrImpl};

pub struct WasmManualDealloc<'a> {
    instance: &'a WasmModuleInstance,
    ptr: WasmPtr<u8>,
    capacity: i32,
}

pub struct WasmBytesSliceResult<'a> {
    pub instance: &'a WasmModuleInstance,
    pub ptr: WasmPtr<u8>,
    pub len: u32,

    pub on_drop_dealloc: Option<(WasmPtr<u8>, i32)>,
}

pub struct WasmBytesSliceOwnedUnsafe {
    pub ptr: WasmPtr<u8>,
    pub len: u32,

    pub on_drop_dealloc: Option<(WasmPtr<u8>, i32)>,
}

impl<'a> WasmBytesSliceResult<'a> {
    pub fn view_to_vec_holder(
        instance: &'a WasmModuleInstance,
        holder: &WasmVecHolder,
    ) -> Result<Self, WasmError> {
        let store = instance.store.try_borrow()?;
        let view = instance.allocation.memory.view(&store);

        let raw_parts = holder.ptr.access(&view)?;
        let raw_parts = raw_parts.as_ref();

        let BytesVecRawParts::<WasmPtrImpl> { ptr, len, .. } = raw_parts.0;

        let len = try_positive_i32_to_u32(len)
            .ok_or_else(|| WasmError::Unspecified(format!("view_to_vec_holder: len {len}")))?;

        Ok(Self {
            instance,
            ptr: ptr.into(),
            len,
            on_drop_dealloc: None,
        })
    }

    pub fn bytes_offset_to_ptr(&self, offset: usize) -> Result<WasmPtr<u8>, WasmError> {
        let offset_u32 = try_usize_to_u32(offset).ok_or_else(|| {
            WasmError::Unspecified(format!("bytes_offset_to_ptr: offset too big {offset}"))
        })?;

        if offset >= u32_to_usize(self.len) {
            return Err(WasmError::Unspecified(format!(
                "bytes_offset_to_ptr: offset {offset}, but len is {}",
                self.len
            )));
        }

        let ptr = self.ptr.add_offset(offset_u32)?;

        Ok(ptr)
    }

    pub fn observe_bytes<T, E: From<WasmError>, F: FnOnce(&[u8]) -> Result<T, E>>(
        &self,
        fun: F,
    ) -> Result<T, Either<E, WasmError>> {
        self.instance.enter_memory_observe_context(|observer| {
            let result = observer
                .observe_byte_slice(self.ptr, self.len, |bytes| fun(bytes))
                .map_err(|either| match either {
                    Either::Left(err) => err,
                    Either::Right(err) => err.into(),
                })?;

            Ok(result)
        })
    }

    pub fn manually_dealloced(&mut self) -> Option<WasmManualDealloc<'_>> {
        self.on_drop_dealloc
            .take()
            .map(|(ptr, capacity)| WasmManualDealloc {
                instance: self.instance,
                ptr,
                capacity,
            })
    }

    pub fn into_owned_unsafe(self) -> WasmBytesSliceOwnedUnsafe {
        WasmBytesSliceOwnedUnsafe {
            ptr: self.ptr,
            len: self.len,
            on_drop_dealloc: self.on_drop_dealloc,
        }
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
