use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use wasmtime::{AsContextMut, Instance, Memory, Store, TypedFunc};
use diffbelt_util::Wrap;

use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtr};
use crate::wasm::{WasmError, WasmStoreData};

pub mod slice;
pub mod vector;

pub enum DeallocType {
    Bytes { ptr: WasmPtr<u8>, len: i32 },
    BytesSlice { ptr: WasmPtr<WasmBytesSlice> },
    VecHolder { ptr: WasmPtr<WasmBytesVecRawParts> },
}

#[derive(Clone)]
pub struct Allocation {
    // FIXME: dealloc them :)
    pub pending_deallocs: Arc<Mutex<Vec<DeallocType>>>,
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
                let $name = instance.get_typed_func(store.as_context_mut(), $name_text)?;
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
            pending_deallocs: Wrap::wrap(Vec::with_capacity(8)),
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
