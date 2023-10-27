use std::marker::PhantomData;
use std::mem::MaybeUninit;
use wasmer::{AsStoreRef, FromToNativeWasmType, WasmPtr};

use diffbelt_wasm_binding::ptr::PtrImpl;
use wasmer_types::{Memory32, MemorySize, ValueType};

use diffbelt_wasm_binding::transform::map_filter::MapFilterResult;
use diffbelt_wasm_binding::{BytesVecFull, ReplaceResult};

#[derive(Copy, Clone, Debug)]
pub struct WasmPtrImpl;

#[derive(Copy, Clone)]
pub struct WasmPtrCopy<T> {
    native: <Memory32 as MemorySize>::Native,
    _phantom: PhantomData<T>,
}

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
    type Ptr<T: Copy> = WasmPtrCopy<T>;
}

pub trait BytesVecFullTrait {
    fn null() -> Self;
}

impl BytesVecFullTrait for BytesVecFull<WasmPtrImpl> {
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
pub struct ReplaceResultWrap(pub ReplaceResult<WasmPtrImpl>);

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct MapFilterResultWrap(pub MapFilterResult<WasmPtrImpl>);

impl_value_type!(ReplaceResultWrap);
impl_value_type!(MapFilterResultWrap);
