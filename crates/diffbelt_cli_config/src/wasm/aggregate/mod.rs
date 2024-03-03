use std::ops::DerefMut;
use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts};
use crate::wasm::{WasmError, WasmModuleInstance};
use diffbelt_protos::protos::transform::aggregate::AggregateMapMultiInput;
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;
use wasmer::{TypedFunction, WasmPtr};
use diffbelt_util_no_std::cast::try_usize_to_i32;

pub struct AggregateFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    bytes_slice: WasmSliceHolder<'a>,
    vector: WasmVecHolder<'a>,
    map: TypedFunction<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    initial_accumulator: TypedFunction<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
    reduce: TypedFunction<
        (
            WasmPtr<u8>,
            i32,
            WasmPtr<u8>,
            i32,
            WasmPtr<WasmBytesVecRawParts>,
        ),
        i32,
    >,
    merge_accumulators: TypedFunction<
        (
            WasmPtr<u8>,
            i32,
            WasmPtr<WasmBytesSlice>,
            i32,
            WasmPtr<WasmBytesVecRawParts>,
        ),
        i32,
    >,
    apply: TypedFunction<
        (
            WasmPtr<u8>,
            i32,
            WasmPtr<u8>,
            i32,
            WasmPtr<WasmBytesVecRawParts>,
        ),
        i32,
    >,
}

impl<'a> AggregateFunctions<'a> {
    pub fn new(
        instance: &'a WasmModuleInstance,
        map: &str,
        initial_accumulator: &str,
        reduce: &str,
        merge_accumulators: &str,
        apply: &str,
    ) -> Result<Self, WasmError> {
        let bytes_slice = instance.alloc_slice_holder()?;
        let vector = instance.alloc_vec_holder()?;

        let store = instance.store.try_borrow()?;

        let map = instance.typed_function_with_store(&store, map)?;
        let initial_accumulator =
            instance.typed_function_with_store(&store, initial_accumulator)?;
        let reduce = instance.typed_function_with_store(&store, reduce)?;
        let merge_accumulators = instance.typed_function_with_store(&store, merge_accumulators)?;
        let apply = instance.typed_function_with_store(&store, apply)?;

        Ok(Self {
            instance,
            bytes_slice,
            vector,
            map,
            initial_accumulator,
            reduce,
            merge_accumulators,
            apply,
        })
    }

    pub fn call_map(
        &self,
        input: FlatbufferAnnotated<&[u8], AggregateMapMultiInput>,
    ) -> Result<(), WasmError> {
        let input = input.value;

        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let input_len = try_usize_to_i32(input.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("AggregateFunctionscall_map: slice len {}", input.len()))
        })?;
        () = self.instance
            .allocation
            .ensure_vec_capacity
            .call(store, self.vector.ptr, input_len)?;

        // TODO

        todo!()
    }
}
