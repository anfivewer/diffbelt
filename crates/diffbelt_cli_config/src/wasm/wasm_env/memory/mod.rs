use wasmer::{Extern, Imports, Memory};
use crate::wasm::wasm_env::WasmEnv;

impl WasmEnv {
    pub fn set_memory(&self, memory: Memory) {
        let mut lock = self.memory.lock().unwrap();
        lock.replace(memory);
    }
}