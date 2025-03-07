use crate::debug_print_string;
use crate::error_code::ErrorCode;
use crate::ptr::bytes::BytesVecRawParts;
use crate::ptr::{ConstPtr, MutPtr};
use alloc::format;
use core::marker::PhantomData;
use diffbelt_protos::align_util::{AlignedBytesError, OwnedAlignedBytes};
use diffbelt_protos::protos::handlers::ApiHandler;
use diffbelt_protos::protos::impls::RequestProto;
use diffbelt_protos::{Serialized, Serializer, FLATBUFFERS_ALIGNMENT};
use diffbelt_util_no_std::cast::{checked_usize_to_u32, u32_to_usize};
use diffbelt_util_no_std::from_either::Either;
use diffbelt_util_no_std::option::store_in_option;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
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
        self.0 > 1
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
    ) -> Result<Self, OwnedAlignedBytes<FLATBUFFERS_ALIGNMENT>> {
        let serializer = A::create_request(serializer, request_args);

        let bytes = serializer.as_bytes();
        let slice_ptr = ConstPtr::from(bytes.as_ptr());

        // SAFETY: trust in host
        let request_id = unsafe { request(slice_ptr, checked_usize_to_u32(bytes.len())) };

        let buffer = serializer.into_aligned_bytes();

        if !request_id.is_valid() {
            return Err(buffer);
        }

        Ok(Self {
            request_id_: request_id,
            buffer: Some(buffer),
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

    pub fn on_request_finished(&mut self) -> Result<Response<'_, A>, ErrorCode> {
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

            return Err(code);
        }

        assert!(response_len > 0, "response length is zero");

        let mut buffer = self
            .buffer
            .take()
            .unwrap_or_else(|| OwnedAlignedBytes::empty());

        let result: Result<_, Either<AlignedBytesError, ErrorCode>> = (|| {
            // SAFETY: trust in host, it should fully initialize buffer bytes, else we will cancel write
            unsafe {
                let mut write = buffer
                    .write_slice(u32_to_usize(response_len))
                    .map_err(Either::Left)?;
                let slice_ptr = write.as_mut();

                let code = copy_response_to(self.request_id_, MutPtr::from(slice_ptr));
                if code.is_error() {
                    self.request_id_ = RequestId::invalid();
                    write.cancel();

                    return Err(Either::Right(code));
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

                return match err {
                    Either::Left(err) => {
                        debug_print_string(format!("{err:?}"));
                        Err(ErrorCode::UnsafeFail)
                    }
                    Either::Right(code) => Err(code),
                };
            }
        }

        let buffer = store_in_option(&mut self.buffer, buffer);
        let slice = buffer.as_slice();

        Ok(Response {
            slice,
            phantom: Default::default(),
        })
    }
}

pub struct Response<'a, A: ApiHandler> {
    slice: &'a [u8],
    phantom: PhantomData<A>,
}
