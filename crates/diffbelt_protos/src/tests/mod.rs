use crate::protos::transform::map_filter::{RecordUpdate, RecordUpdateArgs};
use crate::{OwnedSerialized, SerializedRawParts, Serializer};

#[test]
fn restore_owned() {
    let mut serializer = Serializer::<RecordUpdate>::new();

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

    let SerializedRawParts {
        mut buffer,
        head,
        len,
    } = serialized.into_raw_parts();

    buffer.copy_within(head..(head + len), 0);
    buffer.truncate(len);

    let serialized = OwnedSerialized::<RecordUpdate>::from_vec(buffer).expect("should parse");
    let serialized = serialized.data();

    assert!(serialized.value().is_none());
    assert_eq!(
        serialized.key().expect("no key").bytes(),
        &[1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    )
}
