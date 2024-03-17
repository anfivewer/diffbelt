use crate::wasm::types::WasmPtr;
use crate::wasm::WasmError;
use bytemuck::Pod;

use wasmtime::component::__internal::StoreOpaque;
use wasmtime::{ValRaw, ValType, WasmTy};

pub mod slice;

impl<T: Pod> WasmPtr<T> {
    pub fn null() -> Self {
        Self {
            value: 0,
            phantom: Default::default(),
        }
    }
}

impl<T: Pod> WasmPtr<T> {
    pub fn access<'a>(&self, bytes: &'a [u8]) -> Result<&'a T, WasmError> {
        let slice = self.slice()?;
        slice.at(bytes, 0)
    }

    pub fn read(&self, bytes: &[u8]) -> Result<T, WasmError> {
        let value = self.access(bytes)?;
        Ok(*value)
    }

    pub fn as_mut<'a>(&self, bytes: &'a mut [u8]) -> Result<&'a mut T, WasmError> {
        let slice = self.slice()?;
        slice.at_mut(bytes, 0)
    }

    pub fn write(&self, bytes: &mut [u8], value: T) -> Result<(), WasmError> {
        let m = self.as_mut(bytes)?;
        *m = value;
        Ok(())
    }

    pub fn add_offset(&self, offset: i32) -> Result<Self, WasmError> {
        let ptr = self
            .value
            .checked_add(offset)
            .ok_or_else(|| WasmError::Unspecified(format!("invalid offset {offset}")))?;

        Ok(Self {
            value: ptr,
            phantom: Default::default(),
        })
    }
}

unsafe impl<T: Pod + Send> WasmTy for WasmPtr<T> {
    type Abi = i32;

    fn valtype() -> ValType {
        ValType::I32
    }

    fn compatible_with_store(&self, _store: &StoreOpaque) -> bool {
        true
    }

    fn is_externref(&self) -> bool {
        false
    }

    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        (*raw).get_i32()
    }

    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        *raw = ValRaw::i32(abi);
    }

    fn into_abi(self, _store: &mut StoreOpaque) -> Self::Abi {
        self.value
    }

    unsafe fn from_abi(abi: Self::Abi, _store: &mut StoreOpaque) -> Self {
        Self {
            value: abi,
            phantom: Default::default(),
        }
    }
}
