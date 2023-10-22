use std::cell::RefCell;
use std::io::ErrorKind;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::Utf8Error;

use serde::Deserialize;
use wasmer::{CompileError, ExportError, Function, Imports, Instance, InstantiationError, Memory, MemoryAccessError, MemoryError, MemoryType, Module, RuntimeError, Store, Value};

use crate::errors::WithMark;
use crate::wasm::wasm_env::WasmEnv;

mod wasm_env;

#[derive(Deserialize, Debug)]
pub struct Wasm {
    pub name: Rc<str>,
    pub wasm_path: WithMark<String>,
}

#[derive(Debug)]
pub enum WasmError {
    AlreadyErrored,
    Io(std::io::Error),
    Compile(CompileError),
    Instantiation(InstantiationError),
    Memory(MemoryError),
    Export(ExportError),
    Runtime(RuntimeError),
    MemoryAccess(MemoryAccessError),
    Utf8(Utf8Error),
    MutexPoisoned,
    NoMemory,
    Regex(regex::Error),
    Unspecified(String),
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
}

pub struct MapFilterFunction<'a> {
    instance: &'a WasmModuleInstance,
    fun: &'a Function,
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
            .map_err(WasmError::Export)?;

        env.set_memory(memory.clone());

        Ok(WasmModuleInstance {
            store: RefCell::new(store),
            instance,
        })
    }
}

impl WasmModuleInstance {
    pub fn map_filter_function(&self, name: &str) -> Result<MapFilterFunction<'_>, WasmError> {
        let fun = self
            .instance
            .exports
            .get_function(name)
            .map_err(WasmError::Export)?;

        Ok(MapFilterFunction {
            instance: self,
            fun,
        })
    }
}

impl MapFilterFunction<'_> {
    pub fn call(&self) -> Result<(), WasmError> {
        let mut instance = self.instance.store.borrow_mut();
        let store = instance.deref_mut();

        let result = self.fun.call(store, &[]).map_err(WasmError::Runtime)?;

        println!("result {result:?}");

        Ok(())
    }
}
