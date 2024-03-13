use crate::wasm::types::WasmPtr;
use crate::wasm::WasmError;
use bytemuck::Pod;

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
    pub fn as_mut(&self, bytes: &mut [u8]) -> Result<&mut T, WasmError> {
        let slice = self.slice()?;
        slice.at_mut(bytes, 0)
    }

    pub fn write(&self, bytes: &mut [u8], value: T) -> Result<(), WasmError> {
        let m = self.as_mut(bytes)?;
        *m = value;
    }
}