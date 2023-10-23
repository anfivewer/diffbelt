use crate::wasm::wasm_env::WasmEnv;
use wasmer::Memory;

impl WasmEnv {
    pub fn set_memory(&self, memory: Memory) {
        let mut lock = self.memory.lock().unwrap();
        lock.replace(memory);
    }
}
