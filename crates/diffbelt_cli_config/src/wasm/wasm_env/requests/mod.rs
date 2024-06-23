mod constants;

use either::Either;
use std::collections::HashMap;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::oneshot::Receiver;
use wasmtime::{AsContext, AsContextMut, Caller, Linker, Store};

use crate::requests::DiffbeltRequests;
use diffbelt_protos::align_util::OwnedAlignedBytes;
use diffbelt_protos::protos::api::methods::{Request, Response};
use diffbelt_protos::OwnedSerialized;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util_no_std::cast::{try_usize_to_u32, u32_to_usize};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::requests::RequestId;

use crate::wasm::types::{WasmPtr, WasmPtrToByte, WasmPtrToVecRawParts};
use crate::wasm::wasm_env::requests::constants::ACTIVE_REQUESTS_LIMIT;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmStoreData};

pub struct ActiveDiffbeltRequests {
    requests: HashMap<
        RequestId,
        Either<
            oneshot::Receiver<OwnedSerialized<'static, Response<'static>>>,
            OwnedSerialized<'static, Response<'static>>,
        >,
    >,
    next_id: u32,
}

impl WasmEnv {
    pub fn register_requests_wasm_imports(
        &self,
        store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) -> Result<(), WasmError> {
        {
            let mut state = store.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            state.active_requests = Some(ActiveDiffbeltRequests {
                requests: HashMap::new(),
                next_id: 1,
            });
        }

        async fn request_fn(
            caller: Caller<'_, WasmStoreData>,
            slice_ptr: WasmPtrToByte,
            slice_len: u32,
        ) -> u32 {
            let state_mutex = caller.data().inner.clone();
            let ctx = caller.as_context();

            let result = (|| async move {
                let result = 'block: {
                    let mut state = state_mutex.lock().expect("lock");
                    let state = state.deref_mut();

                    let active_requests =
                        state.active_requests.as_mut().expect("no ActiveRequests");
                    if active_requests.requests.len() >= ACTIVE_REQUESTS_LIMIT {
                        break 'block Either::Left(RequestId::limit_reached());
                    }

                    let memory = state.memory.expect("no memory");
                    let requests = state
                        .requests
                        .as_ref()
                        .expect("no DiffbeltRequests")
                        .clone();

                    let memory = memory.data(ctx);
                    let data = slice_ptr.slice().slice(memory, u32_to_usize(slice_len))?;

                    let buffer = requests.take_request_buffer();
                    let data =
                        OwnedAlignedBytes::copy_slice(buffer, data).map_err(NoStdErrorWrap)?;
                    let data = OwnedSerialized::<Request>::from_aligned_bytes(data)?;

                    Either::Right((requests, data))
                };

                let (requests, data) = match result {
                    Either::Left(request_id) => {
                        return Ok::<_, WasmError>(request_id);
                    }
                    Either::Right(x) => x,
                };

                let receiver = requests
                    .request(data)
                    .await
                    .map_err(|()| WasmError::DiffbeltRequestSend)?;

                let mut state = state_mutex.lock().expect("lock");
                let state = state.deref_mut();
                let mut active_requests = state
                    .active_requests
                    .as_mut()
                    .expect("no ActiveDiffbeltRequests");

                let request_id = RequestId(active_requests.next_id);
                active_requests.next_id += 1;

                active_requests
                    .requests
                    .insert(request_id, Either::Left(receiver));

                Ok::<_, WasmError>(request_id)
            })()
            .await;

            let Some(request_id) = WasmEnv::handle_error(&caller.data().error, result) else {
                return RequestId::invalid().0;
            };

            request_id.0
        }

        fn is_request_finished_fn(caller: Caller<'_, WasmStoreData>, request_id: u32) -> i32 {
            let mut state = caller.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            let active_requests = state.active_requests.as_mut().expect("no ActiveRequests");

            let Some(item) = active_requests.requests.get_mut(&RequestId(request_id)) else {
                return ErrorCode::UnsafeFail.repr();
            };

            match item {
                Either::Left(receiver) => {
                    let value = receiver.try_recv() else {
                        return ErrorCode::UnsafeFail.repr();
                    };

                    match value {
                        Ok(value) => {
                            *item = Either::Right(value);
                            ErrorCode::Ok.repr()
                        }
                        Err(TryRecvError::Empty) => ErrorCode::SafeFail.repr(),
                        Err(TryRecvError::Closed) => ErrorCode::UnsafeFail.repr(),
                    }
                }
                Either::Right(_) => ErrorCode::Ok.repr(),
            }
        }

        async fn on_request_finished_fn(
            mut caller: Caller<'_, WasmStoreData>,
            request_id: u32,
            vec_ptr: WasmPtrToVecRawParts,
            offset_ptr: WasmPtr<u32>,
            len_ptr: WasmPtr<u32>,
        ) -> i32 {
            let state_mutex = caller.data().inner.clone();
            let mut ctx = caller.as_context_mut();

            let result = (|| async move {
                let mut state = state_mutex.lock().expect("lock");
                let state = state.deref_mut();

                let active_requests = state.active_requests.as_mut().expect("no ActiveRequests");

                let Some(item) = active_requests.requests.remove(&RequestId(request_id)) else {
                    return Ok(ErrorCode::UnsafeFail);
                };

                let value = match item {
                    Either::Left(receiver) => {
                        let Ok(value) = receiver.await else {
                            return Ok(ErrorCode::UnsafeFail);
                        };

                        value
                    }
                    Either::Right(value) => value,
                };

                let data_len = value.as_bytes().len();
                let data_len_u32 = try_usize_to_u32(data_len).ok_or_else(|| {
                    WasmError::Unspecified("on_request_finished_fn: data too big".to_string())
                })?;

                let memory = state.memory.as_mut().expect("no Memory");

                let vec_raw_parts = {
                    let memory = memory.data(ctx.as_context());
                    vec_ptr.read(memory)?
                };

                if u32_to_usize(vec_raw_parts.0.capacity) < data_len {
                    let allocation = state.allocation.as_ref().expect("no Allocation");
                    () = allocation
                        .ensure_vec_capacity
                        .call_async(ctx.as_context_mut(), (vec_ptr, data_len_u32))
                        .await?;
                }

                let memory = memory.data_mut(ctx.as_context_mut());
                let mut vec_raw_parts = vec_ptr.read(memory)?;

                vec_raw_parts.0.len = data_len_u32;
                () = vec_raw_parts
                    .0
                    .ptr
                    .slice()
                    .write_slice(memory, value.as_bytes())?;

                () = vec_ptr.write(memory, vec_raw_parts)?;

                Ok(ErrorCode::Ok)
            })()
            .await;

            let Some(error_code) = WasmEnv::handle_error(&caller.data().error, result) else {
                return ErrorCode::UnsafeFail.repr();
            };

            error_code.repr()
        }

        linker.func_wrap2_async(
            "Diffbelt",
            "request",
            |caller: Caller<'_, WasmStoreData>,
             slice_ptr: WasmPtrToByte,
             slice_len: u32|
             -> Box<dyn Future<Output = u32> + Send> {
                Box::new(request_fn(caller, slice_ptr, slice_len))
            },
        )?;
        linker.func_wrap("Diffbelt", "is_request_finished", is_request_finished_fn)?;

        Ok(())
    }
}
