use std::sync::{Arc, Mutex};
use wasmtime::{Linker, Memory, Store};

use diffbelt_util::Wrap;

use crate::wasm::memory::Allocation;
use crate::wasm::{WasmError, WasmStoreData};

pub mod debug;
pub mod memory;
pub mod regex;
mod util;

pub struct WasmEnv {
    error: Arc<Mutex<Option<WasmError>>>,
    memory: Arc<Mutex<Option<Memory>>>,
    allocation: Arc<Mutex<Option<Allocation>>>,
}

impl WasmEnv {
    pub fn new(error: Arc<Mutex<Option<WasmError>>>) -> Self {
        Self {
            error,
            memory: Wrap::wrap(None),
            allocation: Wrap::wrap(None),
        }
    }

    pub fn register_imports(
        &self,
        store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) {
        self.register_debug_wasm_imports(store, linker);
        self.register_regex_wasm_imports(store, linker);
    }

    pub fn handle_error<T>(
        error: impl Into<&Arc<Mutex<Option<WasmError>>>>,
        result: Result<T, WasmError>,
    ) -> Option<T> {
        let wasm_err = match result {
            Ok(x) => {
                return Some(x);
            }
            Err(x) => x,
        };

        let Ok(mut lock) = error.into().try_lock() else {
            // If cannot take mutex, then someone took it to set error
            return None;
        };

        if lock.is_some() {
            return None;
        }

        lock.replace(wasm_err);

        None
    }
}
