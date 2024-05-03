use std::ops::DerefMut;

use wasmtime::{AsContextMut, TypedFunc};

use diffbelt_protos::error::map_flatbuffer_error_to_return_buffer;
use diffbelt_protos::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput, AggregateReduceInput,
    AggregateTargetInfo,
};
use diffbelt_protos::OwnedSerialized;
use diffbelt_util::option::lift_result_from_option;
use diffbelt_util_no_std::cast::{
    checked_positive_i32_to_usize, try_positive_i32_to_usize, try_usize_to_i32,
};
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;
use diffbelt_wasm_binding::error_code::ErrorCode;

use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtr, WasmVecRawParts};
use crate::wasm::{WasmError, WasmModuleInstance};

pub struct AggregateFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    bytes_slice: WasmSliceHolder<'a>,
    input_vector: WasmVecHolder<'a>,
    output_vector: WasmVecHolder<'a>,
    accumulators_vector: WasmPtr<WasmVecRawParts<WasmBytesVecRawParts>>,
    map: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    initial_accumulator: TypedFunc<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
    reduce: TypedFunc<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
    merge_accumulators: Option<
        TypedFunc<
            (
                WasmPtr<WasmBytesVecRawParts>,
                i32,
                WasmPtr<WasmBytesVecRawParts>,
            ),
            i32,
        >,
    >,
    apply: TypedFunc<
        (
            WasmPtr<WasmBytesVecRawParts>,
            WasmPtr<WasmBytesSlice>,
            WasmPtr<WasmBytesVecRawParts>,
        ),
        i32,
    >,
}

impl<'a> AggregateFunctions<'a> {
    pub async fn new(
        instance: &'a WasmModuleInstance,
        map: &str,
        initial_accumulator: &str,
        reduce: &str,
        merge_accumulators: Option<&str>,
        apply: &str,
    ) -> Result<Self, WasmError> {
        let bytes_slice = instance.alloc_slice_holder().await?;
        let input_vector = instance.alloc_vec_holder().await?;
        let output_vector = instance.alloc_vec_holder().await?;

        let mut store = instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let accumulators_vector = instance
            .allocation
            .alloc_vec_raw_parts_of_bytes_vec_raw_parts
            .call_async(store.as_context_mut(), ())
            .await?;

        let map = instance
            .instance
            .get_typed_func(store.as_context_mut(), map)?;
        let initial_accumulator = instance
            .instance
            .get_typed_func(store.as_context_mut(), initial_accumulator)?;
        let reduce = instance
            .instance
            .get_typed_func(store.as_context_mut(), reduce)?;

        let merge_accumulators = merge_accumulators.map(|merge_accumulators| {
            instance
                .instance
                .get_typed_func(store.as_context_mut(), merge_accumulators)
        });
        let merge_accumulators = lift_result_from_option(merge_accumulators)?;

        let apply = instance
            .instance
            .get_typed_func(store.as_context_mut(), apply)?;

        Ok(Self {
            instance,
            bytes_slice,
            input_vector,
            output_vector,
            accumulators_vector,
            map,
            initial_accumulator,
            reduce,
            merge_accumulators,
            apply,
        })
    }

    pub async fn call_map(
        &self,
        input: FlatbufferAnnotated<&[u8], AggregateMapMultiInput<'static>>,
        buffer_holder: &mut Option<Vec<u8>>,
    ) -> Result<OwnedSerialized<'static, AggregateMapMultiOutput<'static>>, WasmError> {
        let wasm_slice = self
            .input_vector
            .replace_with_slice_and_return_slice(input.value)
            .await?;

        {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            {
                let memory = self
                    .instance
                    .allocation
                    .memory
                    .data_mut(store.as_context_mut());
                () = self.bytes_slice.ptr.write(memory, wasm_slice)?;
            }

            let error_code = self
                .map
                .call_async(
                    store.as_context_mut(),
                    (self.bytes_slice.ptr, self.output_vector.ptr),
                )
                .await?;

            let error_code = ErrorCode::from_repr(error_code);
            let ErrorCode::Ok = error_code else {
                return Err(WasmError::Unspecified(format!(
                    "AggregateFunctions::map error code {:?}",
                    error_code
                )));
            };
        }

        let buffer = self.instance.enter_memory_observe_context(|memory| {
            let output = self.bytes_slice.ptr.access(memory)?;
            let output = output.access(memory)?;

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

    pub async fn call_initial_accumulator(
        &self,
        input: FlatbufferAnnotated<&[u8], AggregateTargetInfo<'static>>,
        accumulator_holder: &WasmVecHolder<'a>,
    ) -> Result<(), WasmError> {
        let wasm_slice = self
            .input_vector
            .replace_with_slice_and_return_slice(input.value)
            .await?;

        {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            {
                let memory = self
                    .instance
                    .allocation
                    .memory
                    .data_mut(store.as_context_mut());
                () = self.bytes_slice.ptr.write(memory, wasm_slice)?;
            }

            let error_code = self
                .initial_accumulator
                .call_async(
                    store.as_context_mut(),
                    (wasm_slice.0.ptr, wasm_slice.0.len, accumulator_holder.ptr),
                )
                .await?;

            let error_code = ErrorCode::from_repr(error_code);
            let ErrorCode::Ok = error_code else {
                return Err(WasmError::Unspecified(format!(
                    "AggregateFunctions::initial_accumulator error code {:?}",
                    error_code
                )));
            };
        }

        Ok(())
    }

    pub async fn call_reduce(
        &self,
        input: FlatbufferAnnotated<&[u8], AggregateReduceInput<'static>>,
        accumulator_holder: &WasmVecHolder<'a>,
    ) -> Result<(), WasmError> {
        let wasm_slice = self
            .input_vector
            .replace_with_slice_and_return_slice(input.value)
            .await?;

        {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            {
                let memory = self
                    .instance
                    .allocation
                    .memory
                    .data_mut(store.as_context_mut());
                () = self.bytes_slice.ptr.write(memory, wasm_slice)?;
            }

            let error_code = self
                .reduce
                .call_async(
                    store.as_context_mut(),
                    (wasm_slice.0.ptr, wasm_slice.0.len, accumulator_holder.ptr),
                )
                .await?;

            let error_code = ErrorCode::from_repr(error_code);
            let ErrorCode::Ok = error_code else {
                return Err(WasmError::Unspecified(format!(
                    "AggregateFunctions::reduce error code {:?}",
                    error_code
                )));
            };
        }

        Ok(())
    }

    pub async fn call_merge_accumulators(
        &self,
        input: impl Iterator<Item = impl AsRef<WasmVecHolder<'a>>> + ExactSizeIterator,
        accumulator_holder: &WasmVecHolder<'a>,
    ) -> Result<(), WasmError> {
        let merge_accumulators = self.merge_accumulators.as_ref().ok_or_else(|| {
            WasmError::Unspecified("No merge_accumulator implementation".to_string())
        })?;

        let input_len = try_usize_to_i32(input.len())
            .ok_or_else(|| WasmError::Unspecified("too many accumulators".to_string()))?;

        {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            () = self
                .instance
                .allocation
                .ensure_vec_of_bytes_vec_raw_parts_capacity
                .call_async(
                    store.as_context_mut(),
                    (self.accumulators_vector, input_len),
                )
                .await?;

            let first_accumulator_ptr = {
                let memory = self
                    .instance
                    .allocation
                    .memory
                    .data_mut(store.as_context_mut());

                let mut accumulators_vec = self.accumulators_vector.read(memory)?;
                let mut accumulators_ptr = accumulators_vec.0.ptr;
                let first_accumulator_ptr = accumulators_ptr;

                for accumulator in input {
                    let accumulator = accumulator.as_ref();
                    let raw_parts = accumulator.ptr.read(memory)?;
                    () = accumulators_ptr.write(memory, raw_parts)?;
                    accumulators_ptr = accumulators_ptr.add_offset(1)?;
                }

                accumulators_vec.0.len = input_len;

                () = self.accumulators_vector.write(memory, accumulators_vec)?;

                first_accumulator_ptr
            };

            let error_code = merge_accumulators
                .call_async(
                    store.as_context_mut(),
                    (first_accumulator_ptr, input_len, accumulator_holder.ptr),
                )
                .await?;

            let error_code = ErrorCode::from_repr(error_code);
            let ErrorCode::Ok = error_code else {
                return Err(WasmError::Unspecified(format!(
                    "AggregateFunctions::merge_accumulators error code {:?}",
                    error_code
                )));
            };
        }

        Ok(())
    }

    pub async fn call_apply(
        &self,
        accumulator_holder: &WasmVecHolder<'a>,
    ) -> Result<OwnedSerialized<'static, AggregateApplyOutput<'static>>, WasmError> {
        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let error_code = self
            .apply
            .call_async(
                store.as_context_mut(),
                (
                    accumulator_holder.ptr,
                    self.bytes_slice.ptr,
                    self.output_vector.ptr,
                ),
            )
            .await?;

        let error_code = ErrorCode::from_repr(error_code);
        let ErrorCode::Ok = error_code else {
            return Err(WasmError::AggregateApplyErrorCode(error_code));
        };

        let serialized = {
            let memory = self
                .instance
                .allocation
                .memory
                .data_mut(store.as_context_mut());
            let slice = self.bytes_slice.ptr.read(memory)?;

            let ptr = try_positive_i32_to_usize(slice.0.ptr.value)
                .ok_or_else(|| WasmError::Unspecified("ptr too far".to_string()))?;
            let len = try_positive_i32_to_usize(slice.0.len)
                .ok_or_else(|| WasmError::Unspecified("slice too big".to_string()))?;

            let bytes = &memory[ptr..(ptr + len)];
            let bytes = Vec::from(bytes);

            OwnedSerialized::from_vec(bytes)?
        };

        Ok(serialized)
    }
}
