use crate::protos::api::common::{ErrorResponse, ErrorResponseArgs};
use crate::protos::api::methods::{Response, ResponseArgs, ResponseBody};
use crate::protos::impls::ResponseProto;
use crate::{Serialized, Serializer};
use diffbelt_aligned_bytes::OwnedAlignedBytes;

#[test]
fn test_response_proto() {
    let mut serializer = Serializer::<ResponseProto>::new();
    let details = serializer.create_string("some details");
    let error = ErrorResponse::create(
        serializer.buffer_builder(),
        &ErrorResponseArgs {
            code: 400,
            reason: None,
            details: Some(details),
        },
    );
    let response = Response::create(
        serializer.buffer_builder(),
        &ResponseArgs {
            body_type: ResponseBody::Error,
            body: Some(error.as_union_value()),
        },
    );
    let serialized = serializer.finish(response);
    let serialized = serialized.into_raw_parts();

    let aligned = OwnedAlignedBytes::new(serialized.buffer, serialized.head, serialized.len)
        .expect("aligned bytes");
    let serialized =
        Serialized::<ResponseProto>::from_aligned_bytes(aligned.as_ref()).expect("cannot parse");

    let data = serialized.data();
    assert_eq!(data.body_type(), ResponseBody::Error);
}
