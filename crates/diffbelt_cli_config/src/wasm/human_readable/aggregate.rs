use crate::impl_human_readable_call;
use diffbelt_wasm_binding::error_code::ErrorCode;

use std::ops::DerefMut;
use wasmtime::{AsContextMut, TypedFunc};

use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtr};
use crate::wasm::{WasmError, WasmModuleInstance};

pub struct AggregateHumanReadableFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    slice_holder: WasmSliceHolder<'a>,
    target_key_from_bytes: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    mapped_value_from_bytes:
        TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    accumulator_from_bytes: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
}

impl<'a> AggregateHumanReadableFunctions<'a> {
    pub async fn new(
        instance: &'a WasmModuleInstance,
        target_key_from_bytes: &str,
        mapped_value_from_bytes: &str,
        accumulator_from_bytes: &str,
    ) -> Result<Self, WasmError> {
        let slice_holder = instance.alloc_slice_holder().await?;

        let mut store = instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let target_key_from_bytes = instance
            .instance
            .get_typed_func(store.as_context_mut(), target_key_from_bytes)?;
        let mapped_value_from_bytes = instance
            .instance
            .get_typed_func(store.as_context_mut(), mapped_value_from_bytes)?;
        let accumulator_from_bytes = instance
            .instance
            .get_typed_func(store.as_context_mut(), accumulator_from_bytes)?;

        Ok(Self {
            instance,
            slice_holder,
            target_key_from_bytes,
            mapped_value_from_bytes,
            accumulator_from_bytes,
        })
    }

    impl_human_readable_call!(
        call_target_key_from_bytes,
        target_key_from_bytes,
        "call_target_key_from_bytes"
    );
    impl_human_readable_call!(
        call_mapped_value_from_bytes,
        mapped_value_from_bytes,
        "call_mapped_value_from_bytes"
    );
    impl_human_readable_call!(
        call_accumulator_from_bytes,
        accumulator_from_bytes,
        "call_accumulator_from_bytes"
    );
}
