use crate::wasm::wasm_env::util::ptr_to_utf8;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::WasmError;
use std::sync::{Arc, Mutex};

impl WasmEnv {
    pub fn register_debug_wasm_imports(&self, store: &mut Store, imports: &mut Imports) {
        struct DebugEnv {
            error: Arc<Mutex<Option<WasmError>>>,
            memory: Arc<Mutex<Option<Memory>>>,
        }

        let env = FunctionEnv::new(
            store,
            DebugEnv {
                error: self.error.clone(),
                memory: self.memory.clone(),
            },
        );

        fn print(mut env_mut: FunctionEnvMut<DebugEnv>, s: WasmPtr<u8>, s_size: i32) -> () {
            let (env, store) = env_mut.data_and_store_mut();
            let DebugEnv { error, memory } = env;

            let result = (|| {
                let view = WasmEnv::memory_view(memory, &store)?;

                let s = ptr_to_utf8(&view, s, s_size).unwrap();
                let s = s.as_str().unwrap();

                println!("WASM: {s}");

                Ok::<_, WasmError>(())
            })();

            () = WasmEnv::handle_error(error, result).unwrap_or(());
        }

        imports.define(
            "debug",
            "print",
            Function::new_typed_with_env(store, &env, print),
        );
    }
}
