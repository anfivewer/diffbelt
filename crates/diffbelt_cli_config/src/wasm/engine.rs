use crate::wasm::error::WasmError;
use diffbelt_util::Wrap;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::info;
use wasmtime::{Config, Engine, Module, OptLevel, Strategy};

struct WasmModule {
    name: Arc<str>,
    relative_path: PathBuf,
    wasm_mod: Option<Module>,
}

#[derive(Clone)]
pub struct WasmEngine {
    pub engine: Engine,
    inner: Arc<Mutex<WasmEngineInner>>,
}

pub struct WasmEngineInner {
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
            inner: Wrap::wrap(WasmEngineInner {
                modules: Default::default(),
                wasm_root_path: options.wasm_root_path,
                temp_wasm_path: PathBuf::new(),
            }),
        })
    }

    pub async fn register_module(
        &self,
        name: &str,
        relative_path: PathBuf,
    ) -> Result<(), WasmError> {
        let mut inner = self.inner.lock().await;
        let inner = inner.deref_mut();

        if let Some(module) = inner.modules.get(name) {
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

        inner.modules.insert(name, wasm_module);

        Ok(())
    }

    pub async fn get_module(&self, name: &str) -> Result<Module, WasmError> {
        // TODO: do not lock for so long
        let mut inner = self.inner.lock().await;
        let inner = inner.deref_mut();

        let Some(wasm_module) = inner.modules.get_mut(name) else {
            return Err(WasmError::Unspecified(format!(
                "Module {name} is not registered"
            )));
        };

        if let Some(module) = &wasm_module.wasm_mod {
            return Ok(module.clone());
        }

        let need_capacity = inner.wasm_root_path.as_os_str().len()
            + 1
            + wasm_module.relative_path.as_os_str().len();

        if need_capacity < inner.temp_wasm_path.capacity() {
            inner
                .temp_wasm_path
                .reserve(inner.temp_wasm_path.capacity() - need_capacity);
        }

        inner.temp_wasm_path.clear();
        inner.temp_wasm_path.push(&inner.wasm_root_path);
        inner.temp_wasm_path.push(&wasm_module.relative_path);

        let before = Instant::now();

        let wat_bytes = tokio::fs::read(&inner.temp_wasm_path)
            .await
            .map_err(|err| {
                if let ErrorKind::NotFound = err.kind() {
                    return WasmError::Unspecified(format!(
                        "Did not found wasm file at \"{}\"",
                        inner.temp_wasm_path.to_str().unwrap_or("?")
                    ));
                }

                WasmError::Io(err)
            })?;

        let wasm_mod = Module::new(&self.engine, &wat_bytes)?;

        info!("Wasm file {name} loaded in {:?}", before.elapsed());

        wasm_module.wasm_mod = Some(wasm_mod.clone());

        Ok(wasm_mod)
    }
}
