use crate::wasm::types::WasmPtrToByte;
use crate::wasm::wasm_env::util::ptr_to_utf8;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmModuleInstance, WasmStoreData, WasmStoreDataInner};
use dioxus_hooks::{Ref, RefMut};
use std::ops::DerefMut;
use std::sync::MutexGuard;
use wasmtime::{AsContext, Caller, Linker, Store};

pub struct IntegrationTestsEnv {
    test_error: String,
    has_test_error: bool,
}

impl WasmEnv {
    pub fn register_integration_tests_imports(
        &self,
        store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) -> Result<(), WasmError> {
        {
            let mut state = store.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            state.integration_tests = Some(IntegrationTestsEnv {
                test_error: String::new(),
                has_test_error: false,
            });
        }

        fn set_test_error(caller: Caller<'_, WasmStoreData>, s_ptr: WasmPtrToByte, s_len: u32) {
            let token = {
                let Some(token) = caller.data().non_broken_token() else {
                    return;
                };
                token
            };

            let mut state = caller.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            let memory = state.memory.expect("no memory");
            let integration_tests_env = state
                .integration_tests
                .as_mut()
                .expect("No integration_tests");

            if integration_tests_env.has_test_error {
                return;
            }

            let result = (|| {
                let msg = ptr_to_utf8(caller.as_context(), memory, s_ptr, s_len)?;
                let msg = msg.as_str()?;

                integration_tests_env.has_test_error = true;
                integration_tests_env.test_error.clear();
                integration_tests_env.test_error.push_str(msg);

                Ok(())
            })();

            let Some(()) = WasmEnv::handle_error(&caller.data().error, result, token) else {
                return;
            };
        }

        linker.func_wrap("Diffbelt", "set_test_error", set_test_error)?;

        Ok(())
    }
}

pub struct TestErrorHolder<'a> {
    inner_lock: MutexGuard<'a, WasmStoreDataInner>,
}

impl<'a> TestErrorHolder<'a> {
    /// Can be called only when `TestErrorHolder` lives. On drop it will clear error
    pub fn take(&self) -> Option<&str> {
        let integration_tests = self
            .inner_lock
            .integration_tests
            .as_ref()
            .expect("No integration_tests");

        if !integration_tests.has_test_error {
            return None;
        }

        Some(&integration_tests.test_error)
    }
}

impl<'a> Drop for TestErrorHolder<'a> {
    fn drop(&mut self) {
        let integration_tests = self
            .inner_lock
            .integration_tests
            .as_mut()
            .expect("No integration_tests");
        integration_tests.has_test_error = false;
        integration_tests.test_error.clear();
    }
}

impl WasmStoreData {
    pub fn take_test_error(&mut self) -> Result<TestErrorHolder<'_>, WasmError> {
        let () = self.check_error()?;
        let inner_lock = self.inner.lock().map_err(|_| WasmError::MutexPoisoned)?;
        Ok(TestErrorHolder { inner_lock })
    }
}
