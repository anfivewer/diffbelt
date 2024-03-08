use std::ops::DerefMut;

use diffbelt_protos::error::map_flatbuffer_error_to_return_buffer;
use diffbelt_protos::protos::transform::aggregate::{
    AggregateMapMultiInput, AggregateMapMultiOutput,
};
use diffbelt_protos::OwnedSerialized;
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;
use diffbelt_wasm_binding::error_code::ErrorCode;

use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts};
use crate::wasm::{WasmError, WasmModuleInstance};

pub struct AggregateFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    bytes_slice: WasmSliceHolder<'a>,
    input_vector: WasmVecHolder<'a>,
    output_vector: WasmVecHolder<'a>,
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
        let input_vector = instance.alloc_vec_holder()?;
        let output_vector = instance.alloc_vec_holder()?;

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
            input_vector,
            output_vector,
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
        buffer_holder: &mut Option<Vec<u8>>,
    ) -> Result<OwnedSerialized<AggregateMapMultiOutput>, WasmError> {
        let wasm_slice = self
            .input_vector
            .replace_with_slice_and_return_slice(input.value)?;

        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        {
            let view = self.instance.allocation.memory.view(store);
            () = self.bytes_slice.ptr.write(&view, wasm_slice)?;
        }

        let error_code = self
            .map
            .call(store, self.bytes_slice.ptr, self.output_vector.ptr)?;

        let error_code = ErrorCode::from_repr(error_code);
        let ErrorCode::Ok = error_code else {
            return Err(WasmError::Unspecified(format!(
                "MapFilterFunction error code {:?}",
                error_code
            )));
        };

        let buffer = self.instance.enter_memory_observe_context(|memory| {
            let output = memory.bytes_slice_slice_view(self.bytes_slice.ptr)?;
            let output = output.as_ref();

            let mut buffer = buffer_holder
                .take()
                .unwrap_or_else(|| Vec::with_capacity(output.len()));

            buffer.clear();
            buffer.extend_from_slice(output);

            Ok::<_, WasmError>(buffer)
        })?;

        let result = OwnedSerialized::<AggregateMapMultiOutput>::from_vec(buffer)
            .map_err(map_flatbuffer_error_to_return_buffer(buffer_holder))?;

        Ok(result)
    }
}
