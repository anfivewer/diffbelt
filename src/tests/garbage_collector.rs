use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::create_reader::CreateReaderOptions;
use crate::collection::methods::get::{CollectionGetOk, CollectionGetOptions};
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::collection::CommitGenerationUpdateReader;
use crate::common::{
    KeyValue, KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId,
};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::util::tokio_runtime::create_main_tokio_runtime;
use std::time::Duration;
use tokio::time::sleep;

#[test]
fn garbage_collector_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(garbage_collector_test_inner());
}

async fn garbage_collector_test_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    let _: () = collection
        .create_reader(CreateReaderOptions {
            collection_name: None,
            reader_name: "start".to_string(),
            generation_id: Some(OwnedGenerationId::empty()),
        })
        .await
        .unwrap();

    let first_generation_id =
        OwnedGenerationId::from_boxed_slice(b"0001".to_vec().into_boxed_slice()).unwrap();

    let _: () = collection
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
                    key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Some(OwnedCollectionValue::from_boxed_slice(
                        b"42".to_vec().into_boxed_slice(),
                    )),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice(b"3".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Some(OwnedCollectionValue::from_boxed_slice(
                        b"42".to_vec().into_boxed_slice(),
                    )),
                    if_not_present: false,
                },
            ],
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    let _: () = collection
        .commit_generation(CommitGenerationOptions {
            generation_id: first_generation_id.clone(),
            update_readers: None,
        })
        .await
        .unwrap();

    let CollectionGetOk { item, .. } = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice()).unwrap(),
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(
        item,
        Some(KeyValue {
            key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice()).unwrap(),
            value: OwnedCollectionValue::from_boxed_slice(b"42".to_vec().into_boxed_slice(),),
        })
    );

    let second_generation_id =
        OwnedGenerationId::from_boxed_slice(b"0002".to_vec().into_boxed_slice()).unwrap();

    let _: () = collection
        .start_generation(StartGenerationOptions {
            generation_id: second_generation_id.clone(),
            abort_outdated: false,
        })
        .await
        .unwrap();

    collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Some(OwnedCollectionValue::from_boxed_slice(
                        b"13".to_vec().into_boxed_slice(),
                    )),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice(b"2".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Some(OwnedCollectionValue::from_boxed_slice(
                        b"42".to_vec().into_boxed_slice(),
                    )),
                    if_not_present: false,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice(b"3".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Some(OwnedCollectionValue::from_boxed_slice(
                        b"314".to_vec().into_boxed_slice(),
                    )),
                    if_not_present: false,
                },
            ],
            generation_id: Some(second_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    let _: () = collection
        .commit_generation(CommitGenerationOptions {
            generation_id: second_generation_id.clone(),
            update_readers: Some(vec![CommitGenerationUpdateReader {
                reader_name: "start".to_string(),
                generation_id: second_generation_id.clone(),
            }]),
        })
        .await
        .unwrap();

    // TODO: implement global idle status
    sleep(Duration::from_millis(1000)).await;

    let CollectionGetOk { item, .. } = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice()).unwrap(),
            generation_id: Some(second_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(
        item,
        Some(KeyValue {
            key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice()).unwrap(),
            value: OwnedCollectionValue::from_boxed_slice(b"13".to_vec().into_boxed_slice(),),
        })
    );

    let CollectionGetOk { item, .. } = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"2".to_vec().into_boxed_slice()).unwrap(),
            generation_id: Some(second_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(
        item,
        Some(KeyValue {
            key: OwnedCollectionKey::from_boxed_slice(b"2".to_vec().into_boxed_slice()).unwrap(),
            value: OwnedCollectionValue::from_boxed_slice(b"42".to_vec().into_boxed_slice(),),
        })
    );

    let CollectionGetOk { item, .. } = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"3".to_vec().into_boxed_slice()).unwrap(),
            generation_id: Some(second_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(
        item,
        Some(KeyValue {
            key: OwnedCollectionKey::from_boxed_slice(b"3".to_vec().into_boxed_slice()).unwrap(),
            value: OwnedCollectionValue::from_boxed_slice(b"314".to_vec().into_boxed_slice(),),
        })
    );

    let CollectionGetOk { item, .. } = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice()).unwrap(),
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(item, None);

    let CollectionGetOk { item, .. } = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"3".to_vec().into_boxed_slice()).unwrap(),
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(item, None);
}
