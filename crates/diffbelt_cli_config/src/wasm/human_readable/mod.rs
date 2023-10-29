use wasmer::{TypedFunction, WasmPtr};

use crate::wasm::{WasmError, WasmModuleInstance};
use crate::wasm::types::WasmBytesVecFull;

pub struct HumanReadableFunctions<'a> {
    instance: &'a WasmModuleInstance,
    key_to_bytes: TypedFunction<(WasmPtr<u8>, i32, i32, WasmPtr<WasmBytesVecFull>), i32>,
    bytes_to_key: TypedFunction<(WasmPtr<u8>, i32, i32, WasmPtr<WasmBytesVecFull>), i32>,
    value_to_bytes: TypedFunction<(WasmPtr<u8>, i32, i32, WasmPtr<WasmBytesVecFull>), i32>,
    bytes_to_value: TypedFunction<(WasmPtr<u8>, i32, i32, WasmPtr<WasmBytesVecFull>), i32>,
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
}
