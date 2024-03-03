use std::ops::DerefMut;

use wasmer::{AsStoreRef, Instance, Memory, TypedFunction, WasmPtr};

use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts};
use crate::wasm::{export_error_context, WasmError};

pub mod observe_context;
pub mod vector;
pub mod slice;

#[derive(Clone)]
pub struct Allocation {
    pub alloc: TypedFunction<i32, WasmPtr<u8>>,
    pub dealloc: TypedFunction<(WasmPtr<u8>, i32), ()>,
    alloc_bytes_slice: TypedFunction<(), WasmPtr<WasmBytesSlice>>,
    dealloc_bytes_slice: TypedFunction<WasmPtr<WasmBytesSlice>, ()>,
    pub alloc_bytes_vec_raw_parts: TypedFunction<(), WasmPtr<WasmBytesVecRawParts>>,
    pub dealloc_bytes_vec_raw_parts: TypedFunction<WasmPtr<WasmBytesVecRawParts>, ()>,
    pub ensure_vec_capacity: TypedFunction<(WasmPtr<WasmBytesVecRawParts>, i32), ()>,
    pub memory: Memory,
}

impl Allocation {
    pub fn new(
        store: &(impl AsStoreRef + ?Sized),
        instance: &Instance,
        memory: Memory,
    ) -> Result<Self, WasmError> {
        macro_rules! get_function {
            ($name:ident, $name_text:literal) => {
                let $name = instance
                    .exports
                    .get_typed_function(&store, $name_text)
                    .map_err(export_error_context(|| {
                        concat!($name_text, "()").to_string()
                    }))?;
            };
        }

        get_function!(alloc, "alloc");
        get_function!(dealloc, "dealloc");
        get_function!(alloc_bytes_slice, "alloc_bytes_slice");
        get_function!(dealloc_bytes_slice, "dealloc_bytes_slice");
        get_function!(alloc_bytes_vec_raw_parts, "alloc_bytes_vec_raw_parts");
        get_function!(dealloc_bytes_vec_raw_parts, "dealloc_bytes_vec_raw_parts");
        get_function!(ensure_vec_capacity, "ensure_vec_capacity");

        Ok(Self {
            alloc,
            dealloc,
            alloc_bytes_slice,
            dealloc_bytes_slice,
            alloc_bytes_vec_raw_parts,
            dealloc_bytes_vec_raw_parts,
            ensure_vec_capacity,
            memory,
        })
    }
}
