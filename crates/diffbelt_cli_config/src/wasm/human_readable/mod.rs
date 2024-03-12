pub mod aggregate;

use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use std::ops::DerefMut;
use wasmtime::TypedFunc;

use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::result::WasmBytesSliceResult;
use crate::wasm::types::{WasmBytesSlice, WasmBytesVecRawParts, WasmPtr};
use crate::wasm::{WasmError, WasmModuleInstance, WasmPtrImpl};

pub struct HumanReadableFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    slice_holder: WasmSliceHolder<'a>,
    key_to_bytes: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    bytes_to_key: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    value_to_bytes: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
    bytes_to_value: TypedFunc<(WasmPtr<WasmBytesSlice>, WasmPtr<WasmBytesVecRawParts>), i32>,
}

#[macro_export]
macro_rules! impl_human_readable_call {
    ($fn_name:ident, $field:ident, $fn_name_str:literal) => {
        pub fn $fn_name(
            &self,
            slice: WasmBytesSlice,
            buffer_holder: &WasmVecHolder,
        ) -> Result<WasmBytesSlice, WasmError> {
            let mut store = self.instance.store.try_borrow_mut()?;
            let store = store.deref_mut();

            {
                let view = self.instance.allocation.memory.view(store);
                () = self.slice_holder.ptr.write(&view, slice)?;
            }

            let error_code = self
                .$field
                .call(store, self.slice_holder.ptr, buffer_holder.ptr)?;
            let error_code = ErrorCode::from_repr(error_code);

            let ErrorCode::Ok = error_code else {
                return Err(WasmError::Unspecified(format!(
                    concat!(
                        "HumanReadableFunctions::",
                        $fn_name_str,
                        "() error code {:?}"
                    ),
                    error_code
                )));
            };

            let slice = {
                let view = self.instance.allocation.memory.view(store);
                self.slice_holder.ptr.read(&view)?
            };

            Ok(slice)
        }
    };
}

impl<'a> HumanReadableFunctions<'a> {
    pub fn new(
        instance: &'a WasmModuleInstance,
        key_to_bytes: &str,
        bytes_to_key: &str,
        value_to_bytes: &str,
        bytes_to_value: &str,
    ) -> Result<Self, WasmError> {
        let slice_holder = instance.alloc_slice_holder()?;

        let store = instance.store.try_borrow()?;

        let key_to_bytes = instance.typed_function_with_store(&store, key_to_bytes)?;
        let bytes_to_key = instance.typed_function_with_store(&store, bytes_to_key)?;
        let value_to_bytes = instance.typed_function_with_store(&store, value_to_bytes)?;
        let bytes_to_value = instance.typed_function_with_store(&store, bytes_to_value)?;

        Ok(Self {
            instance,
            slice_holder,
            key_to_bytes,
            bytes_to_key,
            value_to_bytes,
            bytes_to_value,
        })
    }

    impl_human_readable_call!(call_key_to_bytes, key_to_bytes, "call_key_to_bytes");
    impl_human_readable_call!(call_bytes_to_key, bytes_to_key, "call_bytes_to_key");
    impl_human_readable_call!(call_value_to_bytes, value_to_bytes, "call_value_to_bytes");
    impl_human_readable_call!(call_bytes_to_value, bytes_to_value, "call_bytes_to_value");
}
