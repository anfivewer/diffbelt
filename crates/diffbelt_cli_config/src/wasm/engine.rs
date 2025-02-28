use crate::wasm::error::WasmError;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::Instant;
use wasmtime::{Config, Engine, Module, OptLevel, Strategy};

struct WasmModule {
    name: Arc<str>,
    relative_path: PathBuf,
    wasm_mod: Option<Module>,
}

pub struct WasmEngine {
    pub engine: Engine,
    modules: HashMap<Arc<str>, WasmModule>,
    wasm_root_path: PathBuf,
    temp_wasm_path: PathBuf,
}

pub struct WasmEngineOptions {
    pub wasm_root_path: PathBuf,
}

impl WasmEngine {
    pub async fn new(options: WasmEngineOptions) -> Result<Self, WasmError> {
        let mut config = Config::new();
        config.async_support(true);

        // Speedups compilation
        #[cfg(debug_assertions)]
        config.cranelift_pcc(false);

        let engine = Engine::new(&config)?;

        Ok(Self {
            engine,
            modules: Default::default(),
            wasm_root_path: options.wasm_root_path,
            temp_wasm_path: PathBuf::new(),
        })
    }

    pub fn register_module(&mut self, name: &str, relative_path: PathBuf) -> Result<(), WasmError> {
        if let Some(module) = self.modules.get(name) {
            if &module.relative_path == &relative_path {
                return Ok(());
            }

            return Err(
                WasmError::Unspecified(
                    format!(
                        "Module {name} already registered from path {relative_path:?}, but trying to register from {:?}",
                        &module.relative_path
                    )
                ));
        }

        let name: Arc<str> = Arc::from(name);
        let wasm_module = WasmModule {
            name: name.clone(),
            relative_path,
            wasm_mod: None,
        };

        self.modules.insert(name, wasm_module);

        Ok(())
    }

    pub async fn get_module(&mut self, name: &str) -> Result<Module, WasmError> {
        let Some(wasm_module) = self.modules.get_mut(name) else {
            return Err(WasmError::Unspecified(format!(
                "Module {name} is not registered"
            )));
        };

        if let Some(module) = &wasm_module.wasm_mod {
            return Ok(module.clone());
        }

        let need_capacity =
            self.wasm_root_path.as_os_str().len() + 1 + wasm_module.relative_path.as_os_str().len();

        if need_capacity < self.temp_wasm_path.capacity() {
            self.temp_wasm_path
                .reserve(self.temp_wasm_path.capacity() - need_capacity);
        }

        self.temp_wasm_path.clear();
        self.temp_wasm_path.push(&self.wasm_root_path);
        self.temp_wasm_path.push(&wasm_module.relative_path);

        let before = Instant::now();

        let wat_bytes = tokio::fs::read(&self.temp_wasm_path).await.map_err(|err| {
            if let ErrorKind::NotFound = err.kind() {
                return WasmError::Unspecified(format!(
                    "Did not found wasm file at \"{}\"",
                    self.temp_wasm_path.to_str().unwrap_or("?")
                ));
            }

            WasmError::Io(err)
        })?;

        let wasm_mod = Module::new(&self.engine, &wat_bytes)?;

        println!("Loaded wasm file {name} in {:?}", before.elapsed());

        wasm_module.wasm_mod = Some(wasm_mod.clone());

        Ok(wasm_mod)
    }
}
