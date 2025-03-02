use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use wasmtime::{Linker, Memory, Store};

use diffbelt_util::Wrap;

use crate::wasm::error::WasmError;
use crate::wasm::memory::Allocation;
use crate::wasm::{NonBrokenToken, WasmStoreData, WasmStoreErrorState};

pub mod debug;
pub mod integration_tests;
pub mod memory;
pub mod regex;
pub mod requests;
mod util;

pub struct WasmEnv {
    memory: Arc<Mutex<Option<Memory>>>,
    allocation: Arc<Mutex<Option<Allocation>>>,
}

impl WasmEnv {
    pub fn new() -> Self {
        Self {
            memory: Wrap::wrap(None),
            allocation: Wrap::wrap(None),
        }
    }

    pub fn register_imports(
        &self,
        store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) -> Result<(), WasmError> {
        let () = self.register_debug_wasm_imports(linker)?;
        let () = self.register_allocation_imports(store, linker)?;
        let () = self.register_regex_wasm_imports(store, linker)?;
        let () = self.register_requests_wasm_imports(store, linker)?;
        let () = self.register_integration_tests_imports(store, linker)?;
        Ok(())
    }

    pub fn handle_error<T>(
        error: &Arc<Mutex<WasmStoreErrorState>>,
        result: Result<T, WasmError>,
        // Just to trust that we are checked for existing error before reporting about second one
        _non_broken_token: NonBrokenToken,
    ) -> Option<T> {
        let wasm_err = match result {
            Ok(x) => {
                return Some(x);
            }
            Err(WasmError::NonBrokenTokenCheckFail) => {
                return None;
            }
            Err(x) => x,
        };

        let Ok(mut lock) = error.try_lock() else {
            // If cannot take mutex, then someone took it to set error
            return None;
        };

        if lock.error.is_some() {
            return None;
        }

        lock.set_error(wasm_err);

        None
    }
}
