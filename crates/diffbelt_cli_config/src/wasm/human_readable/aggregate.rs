use crate::impl_human_readable_call;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use std::ops::DerefMut;
use wasmtime::TypedFunc;

use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtr};
use crate::wasm::{WasmError, WasmModuleInstance, WasmPtrImpl};

pub struct AggregateHumanReadableFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    slice_holder: WasmSliceHolder<'a>,
    mapped_key_from_bytes: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    mapped_value_from_bytes:
        TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
}

impl<'a> AggregateHumanReadableFunctions<'a> {
    pub fn new(
        instance: &'a WasmModuleInstance,
        mapped_key_from_bytes: &str,
        mapped_value_from_bytes: &str,
    ) -> Result<Self, WasmError> {
        let slice_holder = instance.alloc_slice_holder()?;

        let mut store = instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let mapped_key_from_bytes =
            instance.instance.get_typed_func(store, mapped_key_from_bytes)?;
        let mapped_value_from_bytes =
            instance.instance.get_typed_func(store, mapped_value_from_bytes)?;

        Ok(Self {
            instance,
            slice_holder,
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
