use crate::wasm::types::WasmPtr;
use crate::wasm::WasmError;
use bytemuck::Pod;
use diffbelt_util_no_std::cast::try_positive_i32_to_usize;
use std::marker::PhantomData;
use std::mem;

pub struct WasmSlice<T: Pod> {
    ptr: usize,
    phantom: PhantomData<T>,
}

impl<T: Pod> WasmPtr<T> {
    pub fn slice(&self) -> Result<WasmSlice<T>, WasmError> {
        let ptr = try_positive_i32_to_usize(self.value)
            .ok_or_else(|| WasmError::Unspecified(format!("WasmPtr::slice, ptr {}", self.value)))?;

        Ok(WasmSlice {
            ptr,
            phantom: Default::default(),
        })
    }
}

impl<T: Pod> WasmSlice<T> {
    pub fn at(&self, bytes: &[u8], offset: usize) -> Result<&T, WasmError> {
        let size = mem::size_of::<T>();
        let start = self.ptr + offset * size;
        let m = bytes.get(start..).ok_or_else(|| {
            WasmError::Unspecified(format!("WasmSlice::at, invalid range at {start}"))
        })?;
        let data: &T = bytemuck::cast_ref(m);
        Ok(data)
    }

    pub fn at_mut(&self, bytes: &mut [u8], offset: usize) -> Result<&mut T, WasmError> {
        let size = mem::size_of::<T>();
        let start = self.ptr + offset * size;
        let m = bytes.get_mut(start..).ok_or_else(|| {
            WasmError::Unspecified(format!("WasmSlice::at, invalid range at {start}"))
        })?;
        let data: &mut T = bytemuck::cast_mut(m);
        Ok(data)
    }

    pub fn write_at(&self, bytes: &mut [u8], offset: usize, value: T) -> Result<(), WasmError> {
        let p = self.at_mut(bytes, offset)?;
        *p = value;
        Ok(())
    }

    pub fn write_slice(&self, bytes: &mut [u8], data: &[T]) -> Result<(), WasmError> {
        let m = bytes.get_mut(self.ptr..).ok_or_else(|| {
            WasmError::Unspecified(format!(
                "WasmSlice::write_slice_at, invalid range at {}",
                self.ptr
            ))
        })?;
        let data_slice: &mut [T] = bytemuck::cast_slice_mut(m);
        let data_slice = data_slice.get_mut(..data.len()).ok_or_else(|| {
            WasmError::Unspecified(format!(
                "WasmSlice::write_slice_at, invalid length at {}",
                self.ptr
            ))
        })?;
        data_slice.copy_from_slice(data);
        Ok(())
    }
}