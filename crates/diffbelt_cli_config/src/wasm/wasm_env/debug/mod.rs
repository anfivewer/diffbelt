use crate::wasm::error::WasmError;
use crate::wasm::types::WasmPtr;
use crate::wasm::wasm_env::util::ptr_to_utf8;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::WasmStoreData;
use diffbelt_wasm_binding::error_code::ErrorCode;
use std::ops::DerefMut;
use tracing::debug;
use wasmtime::{AsContext, Caller, Linker};

impl WasmEnv {
    pub fn register_debug_wasm_imports(
        &self,
        linker: &mut Linker<WasmStoreData>,
    ) -> Result<(), WasmError> {
        fn print(caller: Caller<WasmStoreData>, s: WasmPtr<u8>, s_size: u32) -> () {
            let mut state = caller.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            let result = (|| {
                let memory = state.memory.expect("no memory");

                let s = ptr_to_utf8(caller.as_context(), memory, s, s_size).unwrap();
                let s = s.as_str().unwrap();

                debug!("WASM: {s}");

                Ok::<_, WasmError>(())
            })();

            // Checking after execution because it is debug method
            let token = {
                let Some(token) = caller.data().non_broken_token() else {
                    return;
                };
                token
            };

            let () = WasmEnv::handle_error(&caller.data().error, result, token).unwrap_or(());
        }

        linker.func_wrap("debug", "print", print)?;

        Ok(())
    }
}
