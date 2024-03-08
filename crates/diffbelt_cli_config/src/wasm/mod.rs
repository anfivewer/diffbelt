use std::io::ErrorKind;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::Utf8Error;
use std::sync::{Arc, Mutex};

use diffbelt_protos::error::FlatbufferError;
use dioxus_hooks::{BorrowError, BorrowMutError, RefCell};
use serde::Deserialize;
use thiserror::Error;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store, TypedFunc};

use diffbelt_util::Wrap;
use diffbelt_util_no_std::cast::{try_positive_i32_to_u32, try_usize_to_i32, unchecked_i32_to_u32};
use diffbelt_util_no_std::impl_from_either;
use diffbelt_wasm_binding::error_code::ErrorCode;
use memory::Allocation;
pub use types::WasmPtrImpl;

use crate::errors::WithMark;
use crate::wasm::human_readable::HumanReadableFunctions;
use crate::wasm::memory::slice::WasmSliceHolder;
use crate::wasm::result::WasmBytesSliceResult;
use crate::wasm::types::{
    WasmBytesSlice, WasmBytesVecRawParts, WasmPtrToBytesSlice, WasmPtrToVecRawParts,
};
use crate::wasm::wasm_env::WasmEnv;
use memory::vector::WasmVecHolder;

pub mod aggregate;
pub mod human_readable;
pub mod memory;
pub mod result;
pub mod types;
pub mod util;
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
    Utf8(Utf8Error),
    #[error("MutexPoisoned")]
    MutexPoisoned,
    #[error("NoMemory")]
    NoMemory,
    #[error("NoAllocation")]
    NoAllocation,
    #[error("{0:?}")]
    Regex(regex::Error),
    #[error("{0:?}")]
    Borrow(#[from] BorrowError),
    #[error("{0:?}")]
    BorrowMut(#[from] BorrowMutError),
    #[error("{0:?}")]
    Flatbuffer(FlatbufferError),
    #[error("{0:?}")]
    Unspecified(String),
}

impl_from_either!(WasmError);

impl From<FlatbufferError> for WasmError {
    fn from(value: FlatbufferError) -> Self {
        Self::Flatbuffer(value)
    }
}

pub struct NewWasmInstanceOptions<'a> {
    pub config_path: &'a str,
}

pub struct WasmStoreData;

pub struct WasmModuleInstance {
    error: Arc<Mutex<Option<WasmError>>>,
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

        let config = Config::new().async_support(true);
        let engine = Engine::new(&config)?;

        let data = WasmStoreData;

        let mut store = Store::new(&engine, data);
        let wasm_mod = Module::new(&engine, &wat_bytes)?;

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

        let mut linker = Linker::<WasmStoreData>::new(&engine);

        let error: Arc<Mutex<Option<WasmError>>> = Wrap::wrap(None);

        let env = WasmEnv::new(error.clone());

        env.register_imports(&mut store, &mut linker);

        let instance = linker.instantiate_async(&store, &wasm_mod).await?;

        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| WasmError::Unspecified("no memory named \"memory\""))?;

        env.set_memory(memory);

        let allocation = Allocation::new(&store, &instance, memory)?;

        env.set_allocation(allocation.clone());

        Ok(WasmModuleInstance {
            error,
            store: RefCell::new(store),
            instance,
            allocation,
        })
    }
}

impl WasmModuleInstance {
    pub fn map_filter_function(&self, name: &str) -> Result<MapFilterFunction<'_>, WasmError> {
        let slice = self.alloc_slice_holder()?;

        let store = self.store.try_borrow()?;

        let fun = self.typed_function_with_store(&store, name)?;

        Ok(MapFilterFunction {
            instance: self,
            fun,
            slice,
        })
    }

    pub fn human_readable_functions(
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
    }
}

impl MapFilterFunction<'_> {
    /// `inputs` should be encoded by [`diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput`]
    pub fn call(
        &self,
        inputs: &[u8],
        result_buffer: &WasmVecHolder,
    ) -> Result<WasmBytesSliceResult, WasmError> {
        let mut store = self.instance.store.try_borrow_mut()?;
        let store = store.deref_mut();

        let inputs_len_i32 = try_usize_to_i32(inputs.len()).ok_or_else(|| {
            WasmError::Unspecified(format!("Input length too big: {}", inputs.len()))
        })?;

        let ptr = self.instance.allocation.alloc.call(store, inputs_len_i32)?;

        {
            let view = self.instance.allocation.memory.view(store);
            let slice = ptr.slice(&view, unchecked_i32_to_u32(inputs_len_i32))?;
            () = slice.write_slice(inputs)?;

            let mut input_slice_def = self.slice.ptr.access(&view)?;
            let input_slice_def = input_slice_def.as_mut();

            input_slice_def.0.ptr = ptr.into();
            input_slice_def.0.len = inputs_len_i32;
        }

        let error_code = { self.fun.call(store, self.slice.ptr, result_buffer.ptr)? };

        let error_code = ErrorCode::from_repr(error_code);
        let ErrorCode::Ok = error_code else {
            return Err(WasmError::Unspecified(format!(
                "MapFilterFunction error code {:?}",
                error_code
            )));
        };

        let slice_def = {
            let view = self.instance.allocation.memory.view(store);

            let input_slice_def = self.slice.ptr.access(&view)?;
            let input_slice_def = input_slice_def.as_ref();

            *input_slice_def
        };

        let result_len = try_positive_i32_to_u32(slice_def.0.len).ok_or_else(|| {
            WasmError::Unspecified(format!("map_filter call result len: {}", slice_def.0.len))
        })?;

        Ok(WasmBytesSliceResult {
            instance: self.instance,
            ptr: slice_def.0.ptr.into(),
            len: result_len,
        })
    }
}
