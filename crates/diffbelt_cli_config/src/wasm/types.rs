use std::marker::PhantomData;

use bytemuck::{Pod, Zeroable};

use diffbelt_wasm_binding::ptr::bytes::{BytesSlice, BytesVecRawParts};
use diffbelt_wasm_binding::ptr::PtrImpl;
use diffbelt_wasm_binding::ReplaceResult;

#[derive(Pod, Zeroable, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct WasmPtrImpl;

impl PtrImpl for WasmPtrImpl {
    type Ptr<T: Pod> = WasmPtr<T>;
    type MutPtr<T: Pod> = WasmPtr<T>;
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

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(transparent)]
pub struct WasmReplaceResult(pub ReplaceResult<WasmPtrImpl>);

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(transparent)]
pub struct WasmBytesVecRawParts(pub BytesVecRawParts<WasmPtrImpl>);

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(transparent)]
pub struct WasmBytesSlice(pub BytesSlice<WasmPtrImpl>);

impl AsRef<BytesSlice<WasmPtrImpl>> for WasmBytesSlice {
    fn as_ref(&self) -> &BytesSlice<WasmPtrImpl> {
        &self.0
    }
}

#[derive(Pod, Zeroable)]
#[repr(transparent)]
#[derive(Copy)]
pub struct WasmPtr<T: Pod> {
    pub value: i32,
    pub phantom: PhantomData<T>,
}

impl<T: Pod> Clone for WasmPtr<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            phantom: Default::default(),
        }
    }
}

pub type WasmPtrToByte = WasmPtr<u8>;
pub type WasmPtrToBytesSlice = WasmPtr<WasmBytesSlice>;
pub type WasmPtrToVecRawParts = WasmPtr<WasmBytesVecRawParts>;
