use dioxus_hooks::RefCell;
use serde::Deserialize;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use wasmtime::{AsContext, AsContextMut, Instance, Linker, Memory, Module, Store, TypedFunc};

use diffbelt_util::Wrap;
use diffbelt_util_no_std::cast::{try_usize_to_u32, u32_to_usize};
use diffbelt_util_no_std::impl_from_either;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
pub use error::WasmError;
use memory::vector::WasmVecHolder;
use memory::Allocation;
pub use types::WasmPtrImpl;

use crate::errors::WithMark;
use crate::requests::DiffbeltRequests;
use crate::wasm::engine::WasmEngine;
use crate::wasm::human_readable::HumanReadableFunctions;
use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::result::WasmBytesSliceResult;
use crate::wasm::types::{WasmBytesSlice, WasmPtrToBytesSlice, WasmPtrToVecRawParts};
use crate::wasm::wasm_env::integration_tests::IntegrationTestsEnv;
use crate::wasm::wasm_env::memory::AllocationEnv;
use crate::wasm::wasm_env::regex::RegexEnv;
use crate::wasm::wasm_env::requests::ActiveDiffbeltRequests;
use crate::wasm::wasm_env::WasmEnv;

pub mod aggregate;
pub mod engine;
pub mod error;
pub mod human_readable;
pub mod integration_tests;
pub mod memory;
pub mod ptr;
pub mod result;
pub mod types;
pub mod util;
mod wasm_env;

#[derive(Deserialize, Debug)]
pub struct Wasm {
    pub name: Rc<str>,
    pub wasm_path: WithMark<String>,
}

impl_from_either!(WasmError);

pub struct NewWasmInstanceOptions<'a> {
    pub engine: &'a mut WasmEngine,
    pub module: &'a Module,
}

pub struct WasmStoreErrorState {
    // FIXME: check everywhere
    /// Mark this instance as broken, since unrecoverable error happened and memory may be corrupted
    is_broken: Arc<AtomicBool>,
    error: Option<WasmError>,
}

impl WasmStoreErrorState {
    pub fn set_error(&mut self, error: WasmError) {
        self.is_broken.store(true, Ordering::Relaxed);
        self.error = Some(error);
    }
}

pub struct WasmStoreData {
    pub is_broken: Arc<AtomicBool>,
    // FIXME: use it somewhere and check error
    pub error: Arc<Mutex<WasmStoreErrorState>>,
    pub inner: Arc<Mutex<WasmStoreDataInner>>,
}

pub struct WasmStoreDataInner {
    pub memory: Option<Memory>,
    pub allocation: Option<Allocation>,
    pub allocation_env: Option<AllocationEnv>,
    pub regex: Option<RegexEnv>,
    pub requests: Option<Arc<DiffbeltRequests>>,
    pub active_requests: Option<ActiveDiffbeltRequests>,
    pub integration_tests: Option<IntegrationTestsEnv>,
}

#[derive(Copy, Clone)]
pub struct NonBrokenToken {
    inner: (),
}

impl WasmStoreData {
    pub fn new() -> Self {
        let is_broken = Arc::new(AtomicBool::new(false));

        Self {
            is_broken: is_broken.clone(),
            error: Wrap::wrap(WasmStoreErrorState {
                is_broken,
                error: None,
            }),
            inner: Wrap::wrap(WasmStoreDataInner {
                memory: None,
                allocation: None,
                allocation_env: None,
                regex: None,
                requests: None,
                active_requests: None,
                integration_tests: None,
            }),
        }
    }

    pub fn non_broken_token(&self) -> Option<NonBrokenToken> {
        if self.is_broken.load(Ordering::Relaxed) {
            None
        } else {
            Some(NonBrokenToken { inner: () })
        }
    }

    pub fn check_error(&self) -> Result<(), WasmError> {
        if !self.is_broken.load(Ordering::Relaxed) {
            return Ok(());
        }

        // There `is_broken` is true, which means that under lock we are should see error,
        // because is_broken was set under this lock

        let mut lock = self
            .error
            .lock()
            .map_err(|_| WasmError::Unspecified(String::from("Wasm error lock is poisoned")))?;

        let error = lock.error.take();

        if let Some(error) = error {
            return Err(error);
        }

        Err(WasmError::Unspecified(String::from("Wasm store is broken")))
    }
}

pub struct WasmModuleInstance {
    store: RefCell<Store<WasmStoreData>>,
    instance: Instance,
    allocation: Allocation,
}

pub struct MapFilterFunction<'a> {
    pub instance: &'a WasmModuleInstance,
    fun: TypedFunc<(WasmPtrToBytesSlice, WasmPtrToVecRawParts), i32>,
    slice: WasmSliceHolder<'a>,
}

impl Wasm {
    #[deprecated]
    pub async fn new_wasm_instance(
        &self,
        options: NewWasmInstanceOptions<'_>,
    ) -> Result<WasmModuleInstance, WasmError> {
        WasmModuleInstance::new(options).await
    }
}

impl WasmModuleInstance {
    pub async fn new(options: NewWasmInstanceOptions<'_>) -> Result<WasmModuleInstance, WasmError> {
        let NewWasmInstanceOptions { engine, module } = options;

        let data = WasmStoreData::new();

        let mut store = Store::new(&engine.engine, data);
        let mut linker = Linker::<WasmStoreData>::new(&engine.engine);

        let env = WasmEnv::new();

        let () = env.register_imports(&mut store, &mut linker)?;

        let instance = linker.instantiate_async(&mut store, module).await?;

        let mut memory = None;

        for export in instance.exports(&mut store) {
            let Some(m) = export.into_memory() else {
                continue;
            };

            let prev = memory.replace(m);

            if prev.is_some() {
                return Err(WasmError::Unspecified(
                    "Module exports multiple memories".to_string(),
                ));
            }
        }

        let Some(memory) = memory else {
            return Err(WasmError::Unspecified(
                "Module does not exports memory".to_string(),
            ));
        };

        env.set_memory(memory);

        let allocation = Allocation::new(&mut store, &instance, memory)?;

        env.set_allocation(allocation.clone());

        {
            let state = store.data_mut();
            let mut state = state.inner.lock().expect("lock");
            let state = state.deref_mut();

            state.memory = Some(memory);
            state.allocation = Some(allocation.clone());
        }

        Ok(WasmModuleInstance {
            store: RefCell::new(store),
            instance,
            allocation,
        })
    }

    pub async fn map_filter_function(
        &self,
        name: &str,
    ) -> Result<MapFilterFunction<'_>, WasmError> {
        let slice = self.alloc_slice_holder().await?;

        let mut store = self.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let fun = self.instance.get_typed_func(store, name)?;

        Ok(MapFilterFunction {
            instance: self,
            fun,
            slice,
        })
    }

    pub async fn human_readable_functions(
        &self,
        key_to_bytes: &str,
        bytes_to_key: &str,
        value_to_bytes: &str,
        bytes_to_value: &str,
    ) -> Result<HumanReadableFunctions, WasmError> {
        HumanReadableFunctions::new(
            self,
            key_to_bytes,
            bytes_to_key,
            value_to_bytes,
            bytes_to_value,
        )
        .await
    }
}

impl MapFilterFunction<'_> {
    /// `inputs` should be encoded by [`diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput`]
    pub async fn call(
        &self,
        inputs: &[u8],
        result_buffer: &WasmVecHolder<'_>,
    ) -> Result<WasmBytesSliceResult, WasmError> {
        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let inputs_len_u32 = try_usize_to_u32(inputs.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("Input length too big: {}", inputs.len()))
        })?;

        // FIXME: where is dealloc?
        let ptr = self
            .instance
            .allocation
            .alloc
            .call_async(store.as_context_mut(), inputs_len_u32)
            .await?;

        {
            let memory = self
                .instance
                .allocation
                .memory
                .data_mut(store.as_context_mut());
            let ptr_slice = ptr.slice();
            let () = ptr_slice.write_slice(memory, inputs)?;

            let () = self.slice.ptr.write(
                memory,
                WasmBytesSlice(BytesSlice {
                    ptr,
                    len: inputs_len_u32,
                }),
            )?;
        }

        let error_code = {
            self.fun
                .call_async(store.as_context_mut(), (self.slice.ptr, result_buffer.ptr))
                .await?
        };

        let error_code = ErrorCode::from_repr(error_code);
        let ErrorCode::Ok = error_code else {
            return Err(WasmError::Unspecified(format!(
                "MapFilterFunction error code {:?}",
                error_code
            )));
        };

        let slice_def = {
            let memory = self.instance.allocation.memory.data(store.as_context());
            self.slice.ptr.read(memory)?
        };

        let result_len = slice_def.0.len;
        let result_len = u32_to_usize(result_len);

        Ok(WasmBytesSliceResult {
            instance: self.instance,
            ptr: slice_def.0.ptr,
            len: result_len,
        })
    }
}
