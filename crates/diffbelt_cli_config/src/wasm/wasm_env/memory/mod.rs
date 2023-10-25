use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::Allocation;
use wasmer::Memory;

impl WasmEnv {
    pub fn set_memory(&self, memory: Memory) {
        let mut lock = self.memory.lock().unwrap();
        lock.replace(memory);
    }

    pub fn set_allocation(&self, allocation: Allocation) {
        let mut lock = self.allocation.lock().unwrap();
        lock.replace(allocation);
    }
}
