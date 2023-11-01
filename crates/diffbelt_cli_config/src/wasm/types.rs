use std::marker::PhantomData;
use std::mem::MaybeUninit;

use wasmer::WasmPtr;
use wasmer_types::{Memory32, MemorySize, ValueType};

use diffbelt_wasm_binding::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::ptr::PtrImpl;
use diffbelt_wasm_binding::ReplaceResult;
use diffbelt_wasm_binding::transform::map_filter::MapFilterResult;

#[derive(Copy, Clone, Debug)]
pub struct WasmPtrImpl;

#[derive(Clone, Debug)]
pub struct WasmPtrCopy<T> {
    native: <Memory32 as MemorySize>::Native,
    _phantom: PhantomData<T>,
}

impl<T: Clone> Copy for WasmPtrCopy<T> {}

impl<T> From<WasmPtr<T>> for WasmPtrCopy<T> {
    fn from(value: WasmPtr<T>) -> Self {
        Self {
            native: <Memory32 as MemorySize>::offset_to_native(value.offset()),
            _phantom: PhantomData::default(),
        }
    }
}

impl<T> From<WasmPtrCopy<T>> for WasmPtr<T> {
    fn from(value: WasmPtrCopy<T>) -> Self {
        Self::new(<Memory32 as MemorySize>::native_to_offset(value.native))
    }
}

impl PtrImpl for WasmPtrImpl {
    type Ptr<T: Clone> = WasmPtrCopy<T>;
    type MutPtr<T: Clone> = WasmPtrCopy<T>;
}

pub trait BytesVecFullTrait {
    fn null() -> Self;
}

impl BytesVecFullTrait for BytesVecRawParts<WasmPtrImpl> {
    fn null() -> Self {
        Self {
            ptr: WasmPtr::null().into(),
            len: -1,
            capacity: -1,
        }
    }
}

macro_rules! impl_value_type {
    ($name:ident) => {
        unsafe impl ValueType for $name {
            fn zero_padding_bytes(&self, bytes: &mut [MaybeUninit<u8>]) {
                for b in bytes.iter_mut() {
                    b.write(0);
                }
            }
        }
    };
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct WasmReplaceResult(pub ReplaceResult<WasmPtrImpl>);

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct WasmFilterResult(pub MapFilterResult<WasmPtrImpl>);

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct WasmBytesVecRawParts(pub BytesVecRawParts<WasmPtrImpl>);

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct WasmBytesSlice(pub BytesSlice<WasmPtrImpl>);

impl_value_type!(WasmReplaceResult);
impl_value_type!(WasmFilterResult);
impl_value_type!(WasmBytesVecRawParts);
impl_value_type!(WasmBytesSlice);
