use std::cell::RefCell;
use std::io::ErrorKind;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::Utf8Error;

use serde::Deserialize;
use thiserror::Error;
use wasmer::{
    CompileError, ExportError, Imports, Instance, InstantiationError, Memory, MemoryAccessError,
    MemoryError, Module, RuntimeError, Store, TypedFunction, WasmPtr, WasmTypeList,
};

use diffbelt_util::cast::{try_usize_to_i32, unchecked_i32_to_u32};

use crate::errors::WithMark;
use crate::wasm::wasm_env::WasmEnv;

mod wasm_env;

#[derive(Deserialize, Debug)]
pub struct Wasm {
    pub name: Rc<str>,
    pub wasm_path: WithMark<String>,
}

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("AlreadyErrored")]
    AlreadyErrored,
    #[error("{0:?}")]
    Io(std::io::Error),
    #[error("{0:?}")]
    Compile(CompileError),
    #[error("{0:?}")]
    Instantiation(InstantiationError),
    #[error("{0:?}")]
    Memory(MemoryError),
    #[error("{original:?}: {context}")]
    Export {
        original: ExportError,
        context: String,
    },
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("{0:?}")]
    MemoryAccess(MemoryAccessError),
    #[error("{0:?}")]
    Utf8(Utf8Error),
    #[error("MutexPoisoned")]
    MutexPoisoned,
    #[error("NoMemory")]
    NoMemory,
    #[error("{0:?}")]
    Regex(regex::Error),
    #[error("{0:?}")]
    Unspecified(String),
}

pub fn export_error_context<F: FnOnce() -> String>(
    context: F,
) -> impl FnOnce(ExportError) -> WasmError {
    |original| WasmError::Export {
        original,
        context: context(),
    }
}

impl From<MemoryAccessError> for WasmError {
    fn from(value: MemoryAccessError) -> Self {
        Self::MemoryAccess(value)
    }
}

pub struct NewWasmInstanceOptions<'a> {
    pub config_path: &'a str,
}

pub struct WasmModuleInstance {
    store: RefCell<Store>,
    instance: Instance,
    allocation: Allocation,
}

pub struct MapFilterFunction<'a> {
    instance: &'a WasmModuleInstance,
    fun: TypedFunction<(WasmPtr<u8>, i32), ()>,
}

pub struct Allocation {
    alloc: TypedFunction<i32, WasmPtr<u8>>,
    free: TypedFunction<(WasmPtr<u8>, i32), ()>,
    memory: Memory,
}

impl Wasm {
    pub async fn new_wasm_instance(
        &self,
        options: NewWasmInstanceOptions<'_>,
    ) -> Result<WasmModuleInstance, WasmError> {
        let NewWasmInstanceOptions { config_path } = options;

        let mut wasm_path =
            PathBuf::with_capacity(config_path.as_bytes().len() + 1 + self.name.as_bytes().len());
        wasm_path.push(config_path);
        wasm_path.push(self.wasm_path.value.as_str());

        let wat_bytes = tokio::fs::read(&wasm_path).await.map_err(|err| {
            if let ErrorKind::NotFound = err.kind() {
                return WasmError::Unspecified(format!(
                    "Did not found wasm file at \"{}\"",
                    wasm_path.to_str().unwrap_or("?")
                ));
            }

            WasmError::Io(err)
        })?;

        let mut store = Store::default();
        let wasm_mod = Module::new(&store, wat_bytes).map_err(WasmError::Compile)?;

        let mut memories = wasm_mod.exports().memories();
        let Some(memory) = memories.next() else {
            return Err(WasmError::Unspecified(
                "Module does not exports memory".to_string(),
            ));
        };

        if memories.next().is_some() {
            return Err(WasmError::Unspecified(
                "Module exports multiple memories".to_string(),
            ));
        }

        let mut import_object = Imports::new();

        let env = WasmEnv::new();

        env.register_imports(&mut store, &mut import_object);

        let instance = Instance::new(&mut store, &wasm_mod, &import_object)
            .map_err(WasmError::Instantiation)?;

        let memory = instance
            .exports
            .get_memory(memory.name())
            .map_err(export_error_context(|| "memory".to_string()))?;

        env.set_memory(memory.clone());

        let alloc = instance
            .exports
            .get_typed_function(&store, "alloc")
            .map_err(export_error_context(|| "alloc()".to_string()))?;
        let free = instance
            .exports
            .get_typed_function(&store, "free")
            .map_err(export_error_context(|| "free()".to_string()))?;

        let allocation = Allocation {
            alloc,
            free,
            memory: memory.clone(),
        };

        Ok(WasmModuleInstance {
            store: RefCell::new(store),
            instance,
            allocation,
        })
    }
}

impl WasmModuleInstance {
    pub fn map_filter_function(&self, name: &str) -> Result<MapFilterFunction<'_>, WasmError> {
        let store = self.store.borrow();

        let fun = self
            .instance
            .exports
            .get_typed_function(&store, name)
            .map_err(export_error_context(|| format!("map_filter {name}")))?;

        Ok(MapFilterFunction {
            instance: self,
            fun,
        })
    }
}

impl MapFilterFunction<'_> {
    /// `inputs` should be encoded by [`diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput`]
    pub fn call(&self, inputs: &[u8]) -> Result<(), WasmError> {
        let mut store = self.instance.store.borrow_mut();
        let store = store.deref_mut();

        let inputs_len_i32 = try_usize_to_i32(inputs.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("Input length too big: {}", inputs.len()))
        })?;

        let ptr = self.instance.allocation.alloc.call(store, inputs_len_i32)?;

        let view = self.instance.allocation.memory.view(store);
        let slice = ptr.slice(&view, unchecked_i32_to_u32(inputs_len_i32))?;
        () = slice.write_slice(inputs)?;

        let result = self.fun.call(store, ptr, inputs_len_i32)?;

        println!("result {result:?}");

        Ok(())
    }
}
