use crate::wasm::types::{WasmPtr, WasmPtrToByte};
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmStoreData};
use diffbelt_util_no_std::cast::u32_to_usize;
use diffbelt_wasm_binding::error_code::ErrorCode;
use std::fmt::Debug;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::str::from_utf8;
use tracing::debug;
use wasmtime::{AsContext, Caller, Linker, Store};

impl WasmEnv {
    pub fn register_cli_wasm_imports(
        &self,
        _store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) -> Result<(), WasmError> {
        async fn run_transform(
            mut caller: Caller<'_, WasmStoreData>,
            slice_ptr: WasmPtrToByte,
            slice_len: u32,
        ) -> i32 {
            let token = {
                let Some(token) = caller.data().non_broken_token() else {
                    return ErrorCode::UnsafeFail.repr();
                };
                token
            };

            let result = (|| {
                let state = caller.data().inner.lock().expect("lock");
                let state = state.deref();

                let Some(cli_api) = state.cli_api.as_ref() else {
                    return Ok(None);
                };

                let allocation = state.allocation.as_ref().expect("no allocation");
                let memory = allocation.memory.data(caller.as_context());

                let name = slice_ptr.slice().slice(memory, u32_to_usize(slice_len))?;
                let name = from_utf8(name)?;

                Ok(Some((cli_api.clone(), name.to_string())))
            })();

            let Some(result) = WasmEnv::handle_error(&caller.data().error, result, token) else {
                return ErrorCode::UnsafeFail.repr();
            };

            let Some((cli_api, name)) = result else {
                // No cli_api
                return ErrorCode::SafeFail.repr();
            };

            let result = (async move {
                let () = cli_api
                    .run_transform(name.to_string())
                    .await
                    .map_err(|err| {
                        debug!("Run transform {name} failed with: {err:?}");
                        WasmError::RunTransformStringified(format!("{err:?}"))
                    })?;

                Ok::<(), WasmError>(())
            })
            .await;

            let Some(()) = WasmEnv::handle_error(&caller.data().error, result, token) else {
                return ErrorCode::UnsafeFail.repr();
            };

            ErrorCode::Ok.repr()
        }

        linker.func_wrap2_async(
            "Diffbelt",
            "run_transform",
            |caller: Caller<'_, WasmStoreData>,
             slice_ptr: WasmPtrToByte,
             slice_len: u32|
             -> Box<dyn Future<Output = i32> + Send> {
                Box::new(run_transform(caller, slice_ptr, slice_len))
            },
        )?;

        Ok(())
    }
}
