use std::ops::DerefMut;

use wasmtime::{AsContextMut, TypedFunc};

use diffbelt_wasm_binding::error_code::ErrorCode;

use crate::impl_human_readable_call;
use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtr};
use crate::wasm::WasmModuleInstance;
use crate::wasm::error::WasmError;

pub struct AggregateHumanReadableFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    slice_holder: WasmSliceHolder<'a>,
    bytes_to_target_key: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    bytes_to_mapped_value: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    mapped_value_to_bytes: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    bytes_to_accumulator: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    accumulator_to_bytes: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
}

impl<'a> AggregateHumanReadableFunctions<'a> {
    pub async fn new(
        instance: &'a WasmModuleInstance,
        bytes_to_target_key: &str,
        bytes_to_mapped_value: &str,
        mapped_value_to_bytes: &str,
        bytes_to_accumulator: &str,
        accumulator_to_bytes: &str,
    ) -> Result<Self, WasmError> {
        let slice_holder = instance.alloc_slice_holder().await?;

        let mut store = instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let bytes_to_target_key = instance
            .instance
            .get_typed_func(store.as_context_mut(), bytes_to_target_key)?;
        let bytes_to_mapped_value = instance
            .instance
            .get_typed_func(store.as_context_mut(), bytes_to_mapped_value)?;
        let mapped_value_to_bytes = instance
            .instance
            .get_typed_func(store.as_context_mut(), mapped_value_to_bytes)?;
        let bytes_to_accumulator = instance
            .instance
            .get_typed_func(store.as_context_mut(), bytes_to_accumulator)?;
        let accumulator_to_bytes = instance
            .instance
            .get_typed_func(store.as_context_mut(), accumulator_to_bytes)?;

        Ok(Self {
            instance,
            slice_holder,
            bytes_to_target_key,
            bytes_to_mapped_value,
            mapped_value_to_bytes,
            bytes_to_accumulator,
            accumulator_to_bytes,
        })
    }

    impl_human_readable_call!(
        call_bytes_to_target_key,
        bytes_to_target_key,
        "call_bytes_to_target_key"
    );
    impl_human_readable_call!(
        call_bytes_to_mapped_value,
        bytes_to_mapped_value,
        "call_bytes_to_mapped_value"
    );
    impl_human_readable_call!(
        call_mapped_value_to_bytes,
        mapped_value_to_bytes,
        "call_mapped_value_to_bytes"
    );
    impl_human_readable_call!(
        call_bytes_to_accumulator,
        bytes_to_accumulator,
        "call_bytes_to_accumulator"
    );
    impl_human_readable_call!(
        call_accumulator_to_bytes,
        accumulator_to_bytes,
        "call_accumulator_to_bytes"
    );
}
