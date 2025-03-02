use crate::wasm::{WasmError, WasmModuleInstance};
use diffbelt_wasm_binding::error_code::ErrorCode;
use std::ops::DerefMut;
use wasmtime::{AsContextMut, TypedFunc};

pub struct WasmIntegrationTestFunctions<'a> {
    pub instance: &'a WasmModuleInstance,
    test: TypedFunc<(), i32>,
}

impl<'a> WasmIntegrationTestFunctions<'a> {
    pub fn new(instance: &'a WasmModuleInstance, test_fun_name: &str) -> Result<Self, WasmError> {
        let mut store = instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let test = instance
            .instance
            .get_typed_func(store.as_context_mut(), test_fun_name)?;

        Ok(Self { instance, test })
    }

    /// Returns optional test error message
    pub async fn call_test(&self) -> Result<Option<String>, WasmError> {
        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let error_code = self.test.call_async(store.as_context_mut(), ()).await?;

        let error_code = ErrorCode::from_repr(error_code);
        let ErrorCode::Ok = error_code else {
            return Ok(Some(format!("Test exit code {error_code:?}")));
        };

        let data = store.data_mut();

        let test_error = data.take_test_error()?;
        let test_error = test_error.take();

        if let Some(test_error) = test_error {
            return Ok(Some(test_error.to_string()));
        }

        Ok(None)
    }
}
