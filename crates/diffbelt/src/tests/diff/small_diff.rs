use std::io::Write;
use diffbelt_util::debug_print::debug_print;
use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::diff::{DiffOk, DiffOptions, ReadDiffCursorOptions};
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::collection::Collection;
use crate::common::generation_id::GenerationIdSource;
use crate::common::{
    KeyValueDiff, KeyValueUpdate, KeyValueUpdateNewOptions, OwnedCollectionKey,
    OwnedCollectionValue, OwnedGenerationId,
};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::util::tokio_runtime::create_main_tokio_runtime;

const PACK_LIMIT: usize = 20;

#[test]
fn small_diff_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(small_diff_inner());
}

async fn small_diff_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    let first_generation_id = OwnedGenerationId::from_boxed_slice(vec![1u8].into_boxed_slice())
        .expect("invalid generation");

    collection
        .start_generation(StartGenerationOptions {
            generation_id: first_generation_id.clone(),
            abort_outdated: false,
        })
        .await
        .expect("generation");

    collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"k1" as &[u8]).into()).expect(""),
                    value: Some(OwnedCollectionValue::new(b"1")),
                    if_not_present: false,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"k2" as &[u8]).into()).expect(""),
                    value: Some(OwnedCollectionValue::new(b"2")),
                    if_not_present: false,
                }),
            ],
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .expect("put_many");

    collection
        .commit_generation(CommitGenerationOptions {
            generation_id: first_generation_id.clone(),
            update_readers: None,
        })
        .await
        .expect("commit");

    let (from_generation_id, to_generation_id, items) = read_diff(
        &collection,
        DiffOptions {
            from_generation_id: GenerationIdSource::Value(None),
            to_generation_id_loose: None,
        },
    )
    .await;

    assert_eq!(from_generation_id, None);
    assert_eq!(to_generation_id, first_generation_id.clone());

    assert_eq!(
        &items,
        &[
            KeyValueDiff {
                key: OwnedCollectionKey::from_boxed_slice((b"k1" as &[u8]).into()).expect(""),
                from_value: None,
                intermediate_values: vec![],
                to_value: Some(OwnedCollectionValue::new(b"1")),
            },
            KeyValueDiff {
                key: OwnedCollectionKey::from_boxed_slice((b"k2" as &[u8]).into()).expect(""),
                from_value: None,
                intermediate_values: vec![],
                to_value: Some(OwnedCollectionValue::new(b"2")),
            }
        ]
    );

    let second_generation_id = OwnedGenerationId::from_boxed_slice(vec![2u8].into_boxed_slice())
        .expect("invalid generation");

    collection
        .start_generation(StartGenerationOptions {
            generation_id: second_generation_id.clone(),
            abort_outdated: false,
        })
        .await
        .expect("generation");

    collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"k1" as &[u8]).into()).expect(""),
                    value: Some(OwnedCollectionValue::new(b"1-2")),
                    if_not_present: false,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"k2" as &[u8]).into()).expect(""),
                    value: None,
                    if_not_present: false,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"k3" as &[u8]).into()).expect(""),
                    value: Some(OwnedCollectionValue::new(b"3")),
                    if_not_present: false,
                }),
            ],
            generation_id: Some(second_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .expect("put_many");

    collection
        .commit_generation(CommitGenerationOptions {
            generation_id: second_generation_id.clone(),
            update_readers: None,
        })
        .await
        .expect("commit");

    let (from_generation_id, to_generation_id, items) = read_diff(
        &collection,
        DiffOptions {
            from_generation_id: GenerationIdSource::Value(None),
            to_generation_id_loose: None,
        },
    )
        .await;

    assert_eq!(from_generation_id, None);
    assert_eq!(to_generation_id, second_generation_id.clone());

    assert_eq!(
        &items,
        &[
            KeyValueDiff {
                key: OwnedCollectionKey::from_boxed_slice((b"k1" as &[u8]).into()).expect(""),
                from_value: None,
                intermediate_values: vec![],
                to_value: Some(OwnedCollectionValue::new(b"1-2")),
            },
            KeyValueDiff {
                key: OwnedCollectionKey::from_boxed_slice((b"k3" as &[u8]).into()).expect(""),
                from_value: None,
                intermediate_values: vec![],
                to_value: Some(OwnedCollectionValue::new(b"3")),
            }
        ]
    );
}

async fn read_diff(
    collection: &Collection,
    options: DiffOptions,
) -> (
    Option<OwnedGenerationId>,
    OwnedGenerationId,
    Vec<KeyValueDiff>,
) {
    let mut all_items = Vec::new();

    let DiffOk {
        from_generation_id,
        to_generation_id,
        items,
        cursor_id,
    } = collection.diff(options).await.expect("diff");

    all_items.extend(items);

    let mut next_cursor = cursor_id;

    while let Some(cursor_id) = next_cursor.take() {
        let DiffOk {
            from_generation_id: new_from_generation_id,
            to_generation_id: new_to_generation_id,
            items,
            cursor_id,
        } = collection
            .read_diff_cursor(ReadDiffCursorOptions { cursor_id })
            .await
            .expect("read cursor");

        assert_eq!(from_generation_id, new_from_generation_id);
        assert_eq!(to_generation_id, new_to_generation_id);

        next_cursor = cursor_id;

        all_items.extend(items);
    }

    return (from_generation_id, to_generation_id, all_items);
}
