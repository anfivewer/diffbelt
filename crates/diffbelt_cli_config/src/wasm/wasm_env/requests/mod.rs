mod constants;

use diffbelt_http_client::errors::DiffbeltClientError;
use diffbelt_protos::align_util::OwnedAlignedBytes;
use diffbelt_protos::protos::impls::{RequestProto, ResponseProto};
use diffbelt_protos::OwnedSerialized;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util_no_std::cast::{try_usize_to_u32, u32_to_usize};
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_wasm_binding::ptr::MutPtr;
use diffbelt_wasm_binding::requests::RequestId;
use either::Either;
use std::collections::HashMap;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use wasmtime::{AsContext, AsContextMut, Caller, Linker, Store};

use crate::wasm::error::WasmError;
use crate::wasm::types::{WasmPtr, WasmPtrToByte, WasmPtrToVecRawParts};
use crate::wasm::wasm_env::requests::constants::ACTIVE_REQUESTS_LIMIT;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::WasmStoreData;

pub struct ActiveDiffbeltRequests {
    requests: HashMap<
        RequestId,
        Either<
            oneshot::Receiver<Result<OwnedSerialized<ResponseProto>, DiffbeltClientError>>,
            Result<OwnedSerialized<ResponseProto>, DiffbeltClientError>,
        >,
    >,
    next_id: u32,
    current_response: Option<(RequestId, OwnedSerialized<ResponseProto>)>,
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
                current_response: None,
            });
        }

        async fn request_fn(
            caller: Caller<'_, WasmStoreData>,
            slice_ptr: WasmPtrToByte,
            slice_len: u32,
        ) -> u32 {
            let token = {
                let Some(token) = caller.data().non_broken_token() else {
                    return RequestId::invalid().0;
                };
                token
            };

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
                    let data = OwnedSerialized::<RequestProto>::from_aligned_bytes(data)?;

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

            let Some(request_id) = WasmEnv::handle_error(&caller.data().error, result, token)
            else {
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
            len_ptr: WasmPtr<u32>,
        ) -> i32 {
            let token = {
                let Some(token) = caller.data().non_broken_token() else {
                    return ErrorCode::UnsafeFail.repr();
                };
                token
            };

            let state_mutex = caller.data().inner.clone();
            let mut ctx = caller.as_context_mut();

            let result = (|| async move {
                let (item, allocation) = {
                    let mut state = state_mutex.lock().expect("lock");
                    let state = state.deref_mut();

                    let active_requests =
                        state.active_requests.as_mut().expect("no ActiveRequests");

                    let Some(item) = active_requests.requests.remove(&RequestId(request_id)) else {
                        return Ok(ErrorCode::UnsafeFail);
                    };

                    let allocation = state.allocation.clone().expect("no Allocation");

                    (item, allocation)
                };

                let value = match item {
                    Either::Left(receiver) => {
                        let Ok(value) = receiver.await else {
                            return Ok(ErrorCode::UnsafeFail);
                        };

                        if ctx.data().non_broken_token().is_none() {
                            return Ok(ErrorCode::UnsafeFail);
                        }

                        value
                    }
                    Either::Right(value) => value,
                };

                let value = match value {
                    Ok(x) => x,
                    Err(err) => {
                        println!("request error: {err:?}");
                        return Ok(ErrorCode::SafeFail);
                    }
                };

                let data_len = value.as_bytes().len();
                let data_len_u32 = try_usize_to_u32(data_len).ok_or_else(|| {
                    WasmError::Unspecified("on_request_finished_fn: data too big".to_string())
                })?;

                let memory = &allocation.memory;

                let memory = memory.data_mut(ctx.as_context_mut());

                let () = len_ptr.write(memory, data_len_u32)?;

                {
                    let mut state = state_mutex.lock().expect("lock");
                    let state = state.deref_mut();

                    let active_requests =
                        state.active_requests.as_mut().expect("no ActiveRequests");

                    if active_requests.current_response.is_some() {
                        // Previous response should be consumed
                        return Ok(ErrorCode::UnsafeFail);
                    }

                    active_requests.current_response = Some((RequestId(request_id), value));
                }

                Ok(ErrorCode::Ok)
            })()
            .await;

            let Some(error_code) = WasmEnv::handle_error(&caller.data().error, result, token)
            else {
                return ErrorCode::UnsafeFail.repr();
            };

            error_code.repr()
        }

        fn copy_response_to_fn(
            mut caller: Caller<'_, WasmStoreData>,
            request_id: u32,
            head_ptr: WasmPtr<u8>,
        ) -> i32 {
            let token = {
                let Some(token) = caller.data().non_broken_token() else {
                    return ErrorCode::UnsafeFail.repr();
                };
                token
            };

            let (expected_request_id, response, memory) = {
                let mut state = caller.data().inner.lock().expect("lock");
                let state = state.deref_mut();

                let active_requests = state.active_requests.as_mut().expect("no ActiveRequests");

                let Some((expected_request_id, response)) = active_requests.current_response.take()
                else {
                    return ErrorCode::UnsafeFail.repr();
                };

                let memory = state.allocation.as_ref().expect("no allocation").memory;

                (expected_request_id, response, memory)
            };

            if expected_request_id.0 != request_id {
                return ErrorCode::UnsafeFail.repr();
            }

            let memory = memory.data_mut(caller.as_context_mut());
            match head_ptr.slice().write_slice(memory, response.as_bytes()) {
                Ok(()) => {}
                Err(err) => {
                    let _ = WasmEnv::handle_error(&caller.data().error, Err::<(), _>(err), token);
                    return ErrorCode::UnsafeFail.repr();
                }
            }

            ErrorCode::Ok.repr()
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
        linker.func_wrap2_async(
            "Diffbelt",
            "on_request_finished",
            |caller: Caller<'_, WasmStoreData>,
             request_id: u32,
             len_ptr: WasmPtr<u32>|
             -> Box<dyn Future<Output = i32> + Send> {
                Box::new(on_request_finished_fn(caller, request_id, len_ptr))
            },
        )?;
        linker.func_wrap("Diffbelt", "copy_response_to", copy_response_to_fn)?;

        Ok(())
    }
}
