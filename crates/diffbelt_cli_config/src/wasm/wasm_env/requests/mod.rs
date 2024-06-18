use std::collections::HashMap;
use std::future::Future;
use std::ops::DerefMut;

use tokio::sync::oneshot;
use wasmtime::{AsContext, Caller, Linker, Store};

use diffbelt_protos::align_util::OwnedAlignedBytes;
use diffbelt_protos::protos::api::methods::{Request, Response};
use diffbelt_protos::OwnedSerialized;
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util_no_std::cast::u32_to_usize;
use diffbelt_wasm_binding::requests::RequestId;

use crate::wasm::types::WasmPtrToByte;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmStoreData};

pub struct ActiveDiffbeltRequests {
    requests: HashMap<RequestId, oneshot::Receiver<OwnedSerialized<'static, Response<'static>>>>,
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
                let (requests, data) = {
                    let mut state = state_mutex.lock().expect("lock");
                    let state = state.deref_mut();

                    let memory = state.memory.expect("no memory");
                    let requests = state
                        .requests
                        .as_ref()
                        .expect("no DiffbeltRequests")
                        .clone();

                    let memory = memory.data(ctx);
                    let data = slice_ptr.slice()?.slice(memory, u32_to_usize(slice_len))?;

                    let buffer = requests.take_request_buffer();
                    let data =
                        OwnedAlignedBytes::copy_slice(buffer, data).map_err(NoStdErrorWrap)?;
                    let data = OwnedSerialized::<Request>::from_aligned_bytes(data)?;

                    (requests, data)
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

                active_requests.requests.insert(request_id, receiver);

                Ok::<_, WasmError>(request_id)
            })()
            .await;

            let Some(request_id) = WasmEnv::handle_error(&caller.data().error, result) else {
                return RequestId::invalid().0;
            };

            request_id.0
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

        Ok(())
    }
}
