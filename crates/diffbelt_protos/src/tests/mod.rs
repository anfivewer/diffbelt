use crate::align_util::OwnedAlignedBytes;
use crate::protos::impls::RecordUpdateProto;
use crate::protos::transform::map_filter::{RecordUpdate, RecordUpdateArgs};
use crate::{OwnedSerialized, SerializedRawParts, Serializer};

#[test]
fn restore_owned() {
    let mut serializer = Serializer::<RecordUpdateProto>::new();

    let key = serializer.create_vector(&[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);

    let record = RecordUpdate::create(
        serializer.buffer_builder(),
        &RecordUpdateArgs {
            key: Some(key),
            value: None,
        },
    );

    let serialized = serializer.finish(record);
    let serialized = serialized.into_owned();

    let SerializedRawParts { buffer, head, len } = serialized.into_raw_parts();

    let buffer = OwnedAlignedBytes::new(buffer, head, len).expect("align error");

    let serialized =
        OwnedSerialized::<RecordUpdateProto>::from_aligned_bytes(buffer).expect("should parse");
    let serialized = serialized.data();

    assert!(serialized.value().is_none());
    assert_eq!(
        serialized.key().expect("no key").bytes(),
        &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    )
}
