use std::ops::DerefMut;
use wasmtime::{Instance, Memory, Store, TypedFunc};

use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtr};
use crate::wasm::{WasmError, WasmStoreData};

pub mod slice;
pub mod vector;

#[derive(Clone)]
pub struct Allocation {
    pub alloc: TypedFunc<i32, WasmPtr<u8>>,
    pub dealloc: TypedFunc<(WasmPtr<u8>, i32), ()>,
    alloc_bytes_slice: TypedFunc<(), WasmPtr<WasmBytesSlice>>,
    dealloc_bytes_slice: TypedFunc<WasmPtr<WasmBytesSlice>, ()>,
    pub alloc_bytes_vec_raw_parts: TypedFunc<(), WasmPtr<WasmBytesVecRawParts>>,
    pub dealloc_bytes_vec_raw_parts: TypedFunc<WasmPtr<WasmBytesVecRawParts>, ()>,
    pub ensure_vec_capacity: TypedFunc<(WasmPtr<WasmBytesVecRawParts>, i32), ()>,
    pub memory: Memory,
}

impl Allocation {
    pub fn new(
        store: &mut Store<WasmStoreData>,
        instance: &Instance,
        memory: Memory,
    ) -> Result<Self, WasmError> {
        macro_rules! get_function {
            ($name:ident, $name_text:literal) => {
                let $name = instance.get_typed_func(store, $name_text)?;
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
