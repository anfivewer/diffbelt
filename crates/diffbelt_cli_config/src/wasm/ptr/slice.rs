use crate::wasm::types::{WasmBytesSlice, WasmPtr};
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
    pub fn at<'a>(&self, bytes: &'a [u8], offset: usize) -> Result<&'a T, WasmError> {
        let size = mem::size_of::<T>();
        let start = self.ptr + offset * size;
        let m = bytes.get(start..).ok_or_else(|| {
            WasmError::Unspecified(format!("WasmSlice::at, invalid range at {start}"))
        })?;
        let data: &T = bytemuck::from_bytes(m);
        Ok(data)
    }

    pub fn slice<'a>(&self, bytes: &'a [u8], len: usize) -> Result<&'a [T], WasmError> {
        let size = mem::size_of::<T>();
        let start = self.ptr;
        let end = start + len * size;
        let m = bytes.get(start..end).ok_or_else(|| {
            WasmError::Unspecified(format!("WasmSlice::slice, invalid range at {start}..{end}"))
        })?;
        let data: &[T] = bytemuck::cast_slice(m);
        Ok(data)
    }

    pub fn at_mut<'a>(&self, bytes: &'a mut [u8], offset: usize) -> Result<&'a mut T, WasmError> {
        let size = mem::size_of::<T>();
        let start = self.ptr + offset * size;
        let m = bytes.get_mut(start..).ok_or_else(|| {
            WasmError::Unspecified(format!("WasmSlice::at, invalid range at {start}"))
        })?;
        let data: &mut T = bytemuck::from_bytes_mut(m);
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

impl WasmBytesSlice {
    pub fn access<'a>(&self, memory: &'a [u8]) -> Result<&'a [u8], WasmError> {
        let ptr = self.0.ptr;
        let len = self.0.len;
        let len = try_positive_i32_to_usize(len).ok_or_else(|| {
            WasmError::Unspecified(format!("WasmBytesSlice::access, len {}", len))
        })?;
        let slice = ptr.slice()?;
        let slice = slice.slice(memory, len)?;
        Ok(slice)
    }
}
