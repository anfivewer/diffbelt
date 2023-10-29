use diffbelt_wasm_binding::bytes::BytesSlice;
use diffbelt_wasm_binding::error_code::ErrorCode;
use std::ops::DerefMut;
use wasmer::{TypedFunction, WasmPtr};

use crate::wasm::memory::WasmVecHolder;
use crate::wasm::types::WasmBytesVecRawParts;
use crate::wasm::{WasmError, WasmModuleInstance, WasmPtrImpl};

pub struct HumanReadableFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    key_to_bytes: TypedFunction<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
    bytes_to_key: TypedFunction<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
    value_to_bytes: TypedFunction<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
    bytes_to_value: TypedFunction<(WasmPtr<u8>, i32, WasmPtr<WasmBytesVecRawParts>), i32>,
}

macro_rules! impl_call {
    ($fn_name:ident, $field:ident, $fn_name_str:literal) => {
        pub fn $fn_name(
        &self,
        slice: &BytesSlice<WasmPtrImpl>,
        holder: &WasmVecHolder,
    ) -> Result<(), WasmError> {
        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let error_code = self
            .$field
            .call(store, slice.ptr.into(), slice.len, holder.ptr)?;
        let error_code = ErrorCode::from_repr(error_code);

        let ErrorCode::Ok = error_code else {
            return Err(WasmError::Unspecified(format!(
                concat!("HumanReadableFunctions::", $fn_name_str, "() error code {:?}"), error_code
            )));
        };

        Ok(())
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
        let store = instance.store.borrow();

        let key_to_bytes = instance.typed_function_with_store(&store, key_to_bytes)?;
        let bytes_to_key = instance.typed_function_with_store(&store, bytes_to_key)?;
        let value_to_bytes = instance.typed_function_with_store(&store, value_to_bytes)?;
        let bytes_to_value = instance.typed_function_with_store(&store, bytes_to_value)?;

        Ok(Self {
            instance,
            key_to_bytes,
            bytes_to_key,
            value_to_bytes,
            bytes_to_value,
        })
    }

    impl_call!(call_key_to_bytes, key_to_bytes, "call_key_to_bytes");
    impl_call!(call_bytes_to_key, bytes_to_key, "call_bytes_to_key");
    impl_call!(call_value_to_bytes, value_to_bytes, "call_value_to_bytes");
    impl_call!(call_bytes_to_value, bytes_to_value, "call_bytes_to_value");
}
