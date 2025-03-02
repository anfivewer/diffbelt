use crate::wasm::memory::Allocation;
use crate::wasm::types::{WasmBytesVecRawParts, WasmPtr};
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmStoreData};
use std::ops::DerefMut;
use wasmtime::{AsContext, Caller, Linker, Memory, Store};

pub struct AllocationEnv {
    bytes_vec_for_dealloc: Vec<WasmBytesVecRawParts>,
}

impl WasmEnv {
    pub fn set_memory(&self, memory: Memory) {
        let mut lock = self.memory.lock().unwrap();
        lock.replace(memory);
    }

    pub fn set_allocation(&self, allocation: Allocation) {
        let mut lock = self.allocation.lock().unwrap();
        lock.replace(allocation);
    }

    pub fn register_allocation_imports(
        &self,
        store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) -> Result<(), WasmError> {
        {
            let mut state = store.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            state.allocation_env = Some(AllocationEnv {
                bytes_vec_for_dealloc: Vec::new(),
            });
        }

        fn schedule_bytes_vec_dealloc(
            caller: Caller<'_, WasmStoreData>,
            parts_ptr: WasmPtr<WasmBytesVecRawParts>,
        ) {
            let token = {
                let Some(token) = caller.data().non_broken_token() else {
                    return;
                };
                token
            };

            let mut state = caller.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            let memory = state.memory.expect("no memory");
            let allocation_env = state.allocation_env.as_mut().expect("No allocation_env");

            let parts = parts_ptr.access(memory.data(caller.as_context()));
            let Some(parts) = WasmEnv::handle_error(&caller.data().error, parts, token) else {
                return;
            };

            allocation_env.bytes_vec_for_dealloc.push(parts.clone());
        }

        linker.func_wrap(
            "Diffbelt",
            "schedule_bytes_vec_dealloc",
            schedule_bytes_vec_dealloc,
        )?;

        Ok(())
    }
}
