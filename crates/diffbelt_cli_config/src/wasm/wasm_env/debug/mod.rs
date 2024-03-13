use crate::wasm::types::WasmPtr;
use crate::wasm::wasm_env::util::ptr_to_utf8;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmStoreData};
use std::ops::DerefMut;
use wasmtime::{AsContext, Caller, Linker, Store};

impl WasmEnv {
    pub fn register_debug_wasm_imports(
        &self,
        linker: &mut Linker<WasmStoreData>,
    ) {
        fn print(mut caller: Caller<WasmStoreData>, s: WasmPtr<u8>, s_size: i32) -> () {
            let mut state = caller.data().inner.borrow_mut();
            let state = state.deref_mut();

            let result = (|| {
                let memory = state.memory.as_ref().expect("no memory");

                let s = ptr_to_utf8(caller.as_context(), memory, s, s_size).unwrap();
                let s = s.as_str().unwrap();

                println!("WASM: {s}");

                Ok::<_, WasmError>(())
            })();

            () = WasmEnv::handle_error(state.error, result).unwrap_or(());
        }

        linker.func_wrap("debug", "print", print)?;
    }
}
