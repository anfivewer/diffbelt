use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::get_keys_around::{
    CollectionGetKeysAroundOk, CollectionGetKeysAroundOptions,
};
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::query::{QueryOk, QueryOptions};
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::{
    KeyValue, KeyValueUpdate, KeyValueUpdateNewOptions, OwnedCollectionKey, OwnedCollectionValue,
    OwnedGenerationId,
};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::tests::util::query::assert_query;
use crate::util::tokio_runtime::create_main_tokio_runtime;

#[test]
fn generation_bound_phantoms_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(generation_bound_phantoms_inner());
}

async fn generation_bound_phantoms_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    let first_generation_id = OwnedGenerationId::from_boxed_slice((b"0" as &[u8]).into()).unwrap();

    collection
        .start_generation(StartGenerationOptions {
            generation_id: first_generation_id.clone(),
            abort_outdated: false,
        })
        .await
        .unwrap();

    collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"0" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"2" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                }),
            ],
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    let phantom_id = collection.start_phantom().await.unwrap();

    collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"1" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"2" as &[u8]).into()).unwrap(),
                    value: None,
                    if_not_present: false,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"overwrite")),
                    if_not_present: false,
                }),
            ],
            generation_id: Some(first_generation_id.clone()),
            phantom_id: Some(phantom_id.clone()),
        })
        .await
        .unwrap();

    let phantom_id_second = collection.start_phantom().await.unwrap();

    collection
        .put_many(CollectionPutManyOptions {
            items: vec![KeyValueUpdate::new(KeyValueUpdateNewOptions {
                key: OwnedCollectionKey::from_boxed_slice((b"4" as &[u8]).into()).unwrap(),
                value: Some(OwnedCollectionValue::new(b"")),
                if_not_present: false,
            })],
            generation_id: Some(first_generation_id.clone()),
            phantom_id: Some(phantom_id_second.clone()),
        })
        .await
        .unwrap();

    // Query without phantom should not see phantoms
    assert_query(
        collection.as_ref(),
        Some(first_generation_id.clone()),
        None,
        first_generation_id.as_ref(),
        &[
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"0" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"2" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
        ],
    )
    .await;

    // Phantoms see usual records and equal phantom records
    assert_query(
        collection.as_ref(),
        Some(first_generation_id.clone()),
        Some(phantom_id.clone()),
        first_generation_id.as_ref(),
        &[
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"0" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"1" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b"overwrite"),
            },
        ],
    )
    .await;

    // Phantoms see usual records and equal phantom records
    assert_query(
        collection.as_ref(),
        Some(first_generation_id.clone()),
        Some(phantom_id_second.clone()),
        first_generation_id.as_ref(),
        &[
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"0" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"2" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
            KeyValue {
                key: OwnedCollectionKey::from_boxed_slice((b"4" as &[u8]).into()).unwrap(),
                value: OwnedCollectionValue::new(b""),
            },
        ],
    )
    .await;

    collection
        .commit_generation(CommitGenerationOptions {
            generation_id: first_generation_id.clone(),
            update_readers: None,
        })
        .await
        .unwrap();

    // After commit all phantoms should be no more available
    let result = collection
        .query(QueryOptions {
            generation_id: Some(first_generation_id.clone()),
            phantom_id: Some(phantom_id.clone()),
        })
        .await;

    match result {
        Ok(_) => panic!("Success"),
        Err(CollectionMethodError::NoSuchPhantom) => {}
        Err(err) => panic!("Error: {err:?}"),
    }
}
