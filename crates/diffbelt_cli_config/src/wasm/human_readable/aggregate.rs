use crate::impl_human_readable_call;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use std::ops::DerefMut;
use wasmer::{TypedFunction, WasmPtr};

use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::WasmBytesVecRawParts;
use crate::wasm::{WasmError, WasmModuleInstance, WasmPtrImpl};

pub struct AggregateHumanReadableFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    mapped_key_from_bytes: TypedFunction<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
    mapped_value_from_bytes: TypedFunction<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
}

impl<'a> AggregateHumanReadableFunctions<'a> {
    pub fn new(
        instance: &'a WasmModuleInstance,
        mapped_key_from_bytes: &str,
        mapped_value_from_bytes: &str,
    ) -> Result<Self, WasmError> {
        let store = instance.store.try_borrow()?;

        let mapped_key_from_bytes =
            instance.typed_function_with_store(&store, mapped_key_from_bytes)?;
        let mapped_value_from_bytes =
            instance.typed_function_with_store(&store, mapped_value_from_bytes)?;

        Ok(Self {
            instance,
            mapped_key_from_bytes,
            mapped_value_from_bytes,
        })
    }

    impl_human_readable_call!(
        call_mapped_key_from_bytes,
        mapped_key_from_bytes,
        "call_mapped_key_from_bytes"
    );
    impl_human_readable_call!(
        call_mapped_value_from_bytes,
        mapped_value_from_bytes,
        "call_mapped_value_from_bytes"
    );
}
