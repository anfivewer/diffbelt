use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::get::CollectionGetOptions;
use crate::collection::methods::put::{CollectionPutManyOptions, CollectionPutOptions};
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::{
    GenerationId, KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId,
};

use crate::database::create_collection::CreateCollectionOptions;
use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;

use crate::tests::temp_dir::TempDir;
use crate::util::tokio_runtime::create_main_tokio_runtime;

use std::ops::Deref;

use std::time::Duration;

use crate::collection::Collection;
use std::str::from_utf8;
use std::sync::Arc;
use tokio::time::timeout;

#[test]
fn database_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(database_test_inner());
}

async fn database_test_inner() {
    let temp_dir = TempDir::new().unwrap();

    println!("Temp dir: {:?}", temp_dir.get_path_buf());

    let database = Database::open(DatabaseOpenOptions {
        data_path: temp_dir.get_path_buf(),
        config: Arc::new(Default::default()),
    })
    .await
    .expect("Cannot open database");

    let collection = database
        .create_collection("test", CreateCollectionOptions { is_manual: false })
        .await
        .expect("Collection create");

    let manual_collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .expect("Collection create");

    let result = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"test".to_vec().into_boxed_slice()).unwrap(),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.item.is_none());
    let initial_generation_id = result.generation_id;
    assert_eq!(&initial_generation_id, &OwnedGenerationId::zero_64bits());

    let result = collection
        .put(CollectionPutOptions {
            update: KeyValueUpdate {
                key: OwnedCollectionKey::from_boxed_slice(b"test".to_vec().into_boxed_slice())
                    .unwrap(),
                value: Option::Some(OwnedCollectionValue::new(b"passed")),
                if_not_present: true,
            },
            generation_id: None,
            phantom_id: None,
        })
        .await;

    let result = result.unwrap();
    assert_eq!(result.was_put, true);
    let generation_id_after_put = result.generation_id;
    assert!(&generation_id_after_put > &initial_generation_id);

    let mut generation_id_receiver = collection.get_generation_id_receiver();

    loop {
        let is_got_it = {
            let generation_id = generation_id_receiver.borrow_and_update();
            generation_id.deref() >= &generation_id_after_put
        };

        if is_got_it {
            break;
        }

        timeout(Duration::from_millis(100), generation_id_receiver.changed())
            .await
            .unwrap()
            .unwrap();
    }

    assert_get(
        &collection,
        b"test",
        Some("passed"),
        generation_id_after_put.as_ref(),
    )
    .await;

    let result = collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice(b"test".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Option::Some(OwnedCollectionValue::new(b"passed3")),
                    if_not_present: true,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey::from_boxed_slice(b"test2".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Option::Some(OwnedCollectionValue::new(b"passed again")),
                    if_not_present: true,
                },
            ],
            generation_id: None,
            phantom_id: None,
        })
        .await;

    let result = result.unwrap();
    let generation_id_after_put_many = result.generation_id;
    assert!(&generation_id_after_put_many > &generation_id_after_put);

    assert_get(
        &collection,
        b"test",
        Some("passed"),
        generation_id_after_put.as_ref(),
    )
    .await;

    let commit_generation_id =
        OwnedGenerationId::from_boxed_slice(b"first".to_vec().into_boxed_slice()).unwrap();

    let result = manual_collection
        .start_generation(StartGenerationOptions {
            generation_id: commit_generation_id.clone(),
            abort_outdated: false,
        })
        .await;

    assert!(result.is_ok());

    let result = manual_collection
        .put(CollectionPutOptions {
            update: KeyValueUpdate {
                key: OwnedCollectionKey::from_boxed_slice(b"test".to_vec().into_boxed_slice())
                    .unwrap(),
                value: Option::Some(OwnedCollectionValue::new(b"manual passed")),
                if_not_present: true,
            },
            generation_id: Some(
                OwnedGenerationId::from_boxed_slice(b"first".to_vec().into_boxed_slice()).unwrap(),
            ),
            phantom_id: None,
        })
        .await;

    let result = result.unwrap();
    assert!(result.was_put);
    assert_eq!(&commit_generation_id, &result.generation_id);

    assert_get(&manual_collection, b"test", None, GenerationId::empty()).await;

    let result = manual_collection
        .commit_generation(CommitGenerationOptions {
            generation_id: commit_generation_id.clone(),
            update_readers: None,
        })
        .await;

    assert!(result.is_ok());

    let _result = manual_collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(b"test".to_vec().into_boxed_slice()).unwrap(),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    assert_get(
        &manual_collection,
        b"test",
        Some("manual passed"),
        commit_generation_id.as_ref(),
    )
    .await;
}

async fn assert_get(
    collection: &Collection,
    key: &[u8],
    expected_value: Option<&str>,
    expected_generation_id: GenerationId<'_>,
) {
    let result = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey::from_boxed_slice(key.into()).unwrap(),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    let result = result.unwrap();
    assert_eq!(result.generation_id.as_ref(), expected_generation_id);
    assert_eq!(result.item.is_some(), expected_value.is_some());

    match result.item {
        Some(actual_value) => {
            let expected_value = expected_value.unwrap();
            let actual_bytes = actual_value.value.get_value();
            let actual_str = from_utf8(actual_bytes).unwrap();

            assert_eq!(actual_str, expected_value);
        }
        None => {}
    }
}
