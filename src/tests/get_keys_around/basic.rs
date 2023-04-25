use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::get_keys_around::{
    CollectionGetKeysAroundOk, CollectionGetKeysAroundOptions,
};
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::{KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::util::tokio_runtime::create_main_tokio_runtime;

#[test]
fn get_keys_around_basic_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(get_keys_around_basic_inner());
}

async fn get_keys_around_basic_inner() {
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
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice((b"0" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice((b"1" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice((b"2" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice((b"4" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice((b"5" as &[u8]).into()).unwrap(),
                    value: Some(OwnedCollectionValue::new(b"")),
                    if_not_present: false,
                },
            ],
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    collection
        .commit_generation(CommitGenerationOptions {
            generation_id: first_generation_id.clone(),
            update_readers: None,
        })
        .await
        .unwrap();

    let result = collection
        .get_keys_around(CollectionGetKeysAroundOptions {
            key: OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
            generation_id: None,
            phantom_id: None,
            require_key_existance: true,
            limit: 100,
        })
        .await
        .unwrap();

    let CollectionGetKeysAroundOk {
        generation_id,
        left,
        right,
        has_more_on_the_left,
        has_more_on_the_right,
    } = result;

    assert_eq!(&generation_id, &first_generation_id);
    assert_eq!(
        left,
        vec![
            OwnedCollectionKey::from_boxed_slice((b"2" as &[u8]).into()).unwrap(),
            OwnedCollectionKey::from_boxed_slice((b"1" as &[u8]).into()).unwrap(),
            OwnedCollectionKey::from_boxed_slice((b"0" as &[u8]).into()).unwrap(),
        ]
    );
    assert_eq!(
        right,
        vec![
            OwnedCollectionKey::from_boxed_slice((b"4" as &[u8]).into()).unwrap(),
            OwnedCollectionKey::from_boxed_slice((b"5" as &[u8]).into()).unwrap(),
        ]
    );
    assert!(!has_more_on_the_left);
    assert!(!has_more_on_the_right);

    let result = collection
        .get_keys_around(CollectionGetKeysAroundOptions {
            key: OwnedCollectionKey::from_boxed_slice((b"1" as &[u8]).into()).unwrap(),
            generation_id: None,
            phantom_id: None,
            require_key_existance: true,
            limit: 2,
        })
        .await
        .unwrap();

    let CollectionGetKeysAroundOk {
        generation_id,
        left,
        right,
        has_more_on_the_left,
        has_more_on_the_right,
    } = result;

    assert_eq!(&generation_id, &first_generation_id);
    assert_eq!(
        left,
        vec![OwnedCollectionKey::from_boxed_slice((b"0" as &[u8]).into()).unwrap(),]
    );
    assert_eq!(
        right,
        vec![
            OwnedCollectionKey::from_boxed_slice((b"2" as &[u8]).into()).unwrap(),
            OwnedCollectionKey::from_boxed_slice((b"3" as &[u8]).into()).unwrap(),
        ]
    );
    assert!(!has_more_on_the_left);
    assert!(has_more_on_the_right);
}
