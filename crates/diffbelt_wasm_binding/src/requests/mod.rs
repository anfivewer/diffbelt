pub mod errors;

use crate::error_code::ErrorCode;
use crate::ptr::{ConstPtr, MutPtr};
use crate::requests::errors::RequestErrorWithBuffer;
use alloc::string::String;
use core::marker::PhantomData;
use diffbelt_protos::align_util::{AlignedBytes, OwnedAlignedBytes};
use diffbelt_protos::protos::handlers::ApiHandler;
use diffbelt_protos::protos::impls::{RequestProto, ResponseProto};
use diffbelt_protos::{
    FlatbuffersGenericType, OwnedSerialized, Serialized, Serializer, FLATBUFFERS_ALIGNMENT,
};
use diffbelt_util_no_std::cast::{checked_usize_to_u32, u32_to_usize};
use diffbelt_util_no_std::option::store_in_option;
use diffbelt_wasm_binding::requests::errors::RequestError;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(C)]
pub struct RequestId(pub u32);

impl RequestId {
    pub fn invalid() -> Self {
        Self(0)
    }

    pub fn limit_reached() -> Self {
        Self(1)
    }

    pub fn is_valid(&self) -> bool {
        self.0 >= 64
    }
}

#[link(wasm_import_module = "Diffbelt")]
extern "C" {
    fn request(slice_ptr: ConstPtr<u8>, slice_len: u32) -> RequestId;
    fn is_request_finished(request_id: RequestId) -> ErrorCode;
    /// On success writes length of response in bytes to `len_ptr`
    fn on_request_finished(request_id: RequestId, len_ptr: MutPtr<u32>) -> ErrorCode;
    /// Can be called only after `on_request_finished`, writes response starting from `head_ptr`
    /// with length previously written to `len_ptr`
    fn copy_response_to(request_id: RequestId, head_ptr: MutPtr<u8>) -> ErrorCode;
}

pub struct Request<A: ApiHandler> {
    request_id_: RequestId,
    buffer: Option<OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>>,
    phantom: PhantomData<A>,
}

impl<A: ApiHandler> Request<A> {
    pub fn call<'a>(
        serializer: Serializer<'a, RequestProto>,
        request_args: A::FlatbuffersRequestArgs<'a>,
    ) -> Result<Self, RequestErrorWithBuffer> {
        let serializer = A::create_request(serializer, request_args);

        let bytes = serializer.as_bytes();
        let slice_ptr = ConstPtr::from(bytes.as_ptr());

        // SAFETY: trust in host
        let request_id = unsafe { request(slice_ptr, checked_usize_to_u32(bytes.len())) };

        let buffer = serializer.into_aligned_bytes();

        if !request_id.is_valid() {
            return Err(RequestErrorWithBuffer {
                buffer: Some(buffer),
                error: RequestError::HostCall(ErrorCode::SafeFail),
            });
        }

        Ok(Self {
            request_id_: request_id,
            buffer: Some(buffer),
            phantom: Default::default(),
        })
    }

    pub fn call_raw(
        request_data: Serialized<'_, RequestProto>,
    ) -> Result<Self, RequestErrorWithBuffer> {
        let bytes = request_data.as_bytes();
        let slice_ptr = ConstPtr::from(bytes.as_ptr());

        // SAFETY: trust in host
        let request_id = unsafe { request(slice_ptr, checked_usize_to_u32(bytes.len())) };

        if !request_id.is_valid() {
            return Err(RequestErrorWithBuffer {
                buffer: None,
                error: RequestError::HostCall(ErrorCode::SafeFail),
            });
        }

        Ok(Self {
            request_id_: request_id,
            buffer: None,
            phantom: Default::default(),
        })
    }

    pub fn request_id(&self) -> RequestId {
        self.request_id_
    }

    pub fn take_buffer(&mut self) -> Option<OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>> {
        self.buffer.take()
    }

    pub fn replace_buffer(
        &mut self,
        buffer: OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>,
    ) -> Option<OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>> {
        self.buffer.replace(buffer)
    }

    pub fn is_request_finished(&self) -> Result<(), ErrorCode> {
        // SAFETY: trust in host
        let code = unsafe { is_request_finished(self.request_id_) };

        match code {
            ErrorCode::Ok => Ok(()),
            _ => Err(code),
        }
    }

    pub fn on_request_finished(&mut self) -> Result<Response<'_, A>, RequestError> {
        let mut response_len = 0u32;
        // SAFETY: trust in host
        let code = unsafe {
            on_request_finished(
                self.request_id_,
                MutPtr::from(&mut response_len as *mut u32),
            )
        };
        if code.is_error() {
            self.request_id_ = RequestId::invalid();

            return Err(RequestError::HostCall(code));
        }

        assert!(response_len > 0, "response length is zero");

        let mut buffer = self
            .buffer
            .take()
            .unwrap_or_else(|| OwnedAlignedBytes::empty());

        let result: Result<_, RequestError> = (|| {
            // SAFETY: trust in host, it should fully initialize buffer bytes, else we will cancel write
            unsafe {
                let mut write = buffer.write_slice(u32_to_usize(response_len))?;
                let slice_ptr = write.as_mut();

                let code = copy_response_to(self.request_id_, MutPtr::from(slice_ptr));
                if code.is_error() {
                    self.request_id_ = RequestId::invalid();
                    write.cancel();

                    return Err(RequestError::HostCall(code));
                }

                // Drop will resize buffer and we should have correct slice in it
                drop(write);
            }

            Ok(())
        })();

        match result {
            Ok(()) => {}
            Err(err) => {
                self.buffer = Some(buffer);

                return Err(err);
            }
        }

        let buffer = store_in_option(&mut self.buffer, buffer);
        let bytes = buffer.as_ref();

        Ok(Response {
            bytes,
            phantom: Default::default(),
        })
    }
}

pub struct Response<'a, A: ApiHandler> {
    bytes: AlignedBytes<'a, FLATBUFFERS_ALIGNMENT>,
    phantom: PhantomData<A>,
}

impl<'a, A: ApiHandler> Response<'a, A> {
    pub fn response(
        &self,
    ) -> Result<<A::FlatbuffersResponse as FlatbuffersGenericType>::FlatType<'a>, RequestError>
    {
        let serialized = Serialized::<ResponseProto>::from_aligned_bytes(self.bytes)?;
        let response = serialized.data();

        if let Some(error) = response.body_as_error() {
            return Err(RequestError::Response {
                code: error.code(),
                reason: error.reason().map(String::from),
                details: error.details().map(String::from),
            });
        }

        let body =
            A::response(response).ok_or_else(|| RequestError::MessageStatic("no response body"))?;
        Ok(body)
    }
}
