use std::collections::BTreeMap;

use futures::future::BoxFuture;

use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::query::{QueryOk, QueryOptions, ReadQueryCursorOptions};
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::collection::Collection;
use crate::common::{
    GenerationId, KeyValue, KeyValueUpdate, KeyValueUpdateNewOptions, OwnedCollectionKey,
    OwnedCollectionValue, OwnedGenerationId,
};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::tests::util::query::assert_query;
use crate::util::tokio_runtime::create_main_tokio_runtime;

#[test]
fn query_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(query_test_inner());
}

async fn query_test_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    let first_generation_id =
        OwnedGenerationId::from_boxed_slice(b"0".to_vec().into_boxed_slice()).unwrap();

    collection
        .start_generation(StartGenerationOptions {
            generation_id: first_generation_id.clone(),
            abort_outdated: false,
        })
        .await
        .unwrap();

    let mut first_generation_updates = Vec::with_capacity(100);

    for i in 0..100 {
        let key = [(i % 256) as u8];
        let value = [(i % 2) as u8, (i % 3) as u8, (i % 4) as u8, (i % 5) as u8];

        first_generation_updates.push(KeyValueUpdate::new(KeyValueUpdateNewOptions {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: Some(OwnedCollectionValue::new(&value)),
            if_not_present: false,
        }));
    }

    let result = collection
        .put_many(CollectionPutManyOptions {
            items: first_generation_updates.clone(),
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(result.generation_id, first_generation_id);

    collection
        .commit_generation(CommitGenerationOptions {
            generation_id: first_generation_id.clone(),
            update_readers: None,
        })
        .await
        .unwrap();

    let first_generation_expected_items =
        key_value_update_items_to_key_value(first_generation_updates.clone());

    assert_query(
        collection.as_ref(),
        None,
        None,
        first_generation_id.as_ref(),
        &first_generation_expected_items,
    )
    .await;

    let second_generation_id =
        OwnedGenerationId::from_boxed_slice(b"1".to_vec().into_boxed_slice()).unwrap();

    collection
        .start_generation(StartGenerationOptions {
            generation_id: second_generation_id.clone(),
            abort_outdated: false,
        })
        .await
        .unwrap();

    let mut second_generation_updates = Vec::with_capacity(300);

    // 30 items we will update
    for i in 0..30 {
        let key = [(i % 256) as u8];

        let i = i + 1;
        let value = [(i % 2) as u8, (i % 3) as u8, (i % 4) as u8, (i % 5) as u8];

        second_generation_updates.push(KeyValueUpdate::new(KeyValueUpdateNewOptions {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: Some(OwnedCollectionValue::new(&value)),
            if_not_present: false,
        }));
    }

    // 20 items we will remove
    for i in 30..50 {
        let key = [(i % 256) as u8];

        second_generation_updates.push(KeyValueUpdate::new(KeyValueUpdateNewOptions {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: None,
            if_not_present: false,
        }));
    }

    // 250 items will be added
    for i in 100..350 {
        let key = [(i % 256) as u8, (i % 29) as u8];
        let value = [(i % 2) as u8, (i % 3) as u8, (i % 4) as u8, (i % 5) as u8];

        second_generation_updates.push(KeyValueUpdate::new(KeyValueUpdateNewOptions {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: Some(OwnedCollectionValue::new(&value)),
            if_not_present: false,
        }));
    }

    let result = collection
        .put_many(CollectionPutManyOptions {
            items: second_generation_updates.clone(),
            generation_id: Some(second_generation_id.clone()),
            phantom_id: None,
        })
        .await
        .unwrap();

    assert_eq!(result.generation_id, second_generation_id);

    collection
        .commit_generation(CommitGenerationOptions {
            generation_id: second_generation_id.clone(),
            update_readers: None,
        })
        .await
        .unwrap();

    // First generation should not be changed
    assert_query(
        collection.as_ref(),
        Some(first_generation_id.clone()),
        None,
        first_generation_id.as_ref(),
        &first_generation_expected_items,
    )
    .await;

    let second_generation_expected_items =
        merge_kv_updates(vec![&first_generation_updates, &second_generation_updates]);

    assert_query(
        collection.as_ref(),
        None,
        None,
        second_generation_id.as_ref(),
        &second_generation_expected_items,
    )
    .await;

    let query_cursors_count = collection.query_cursors_count().await;
    assert_eq!(query_cursors_count, 1);
}

fn key_value_update_items_to_key_value(items: Vec<KeyValueUpdate>) -> Vec<KeyValue> {
    let mut items: Vec<KeyValue> = items
        .into_iter()
        .filter_map(|key_value_update| {
            key_value_update.value.map(|value| KeyValue {
                key: key_value_update.key.into_owned(),
                value,
            })
        })
        .collect();

    items.sort_by(|a, b| a.key.cmp(&b.key));

    items
}

fn merge_kv_updates(updates_list: Vec<&Vec<KeyValueUpdate>>) -> Vec<KeyValue> {
    let mut map = BTreeMap::new();

    for updates in updates_list {
        for kv_update in updates {
            match &kv_update.value {
                Some(value) => {
                    map.insert(kv_update.key.clone(), value.clone());
                }
                None => {
                    map.remove(&kv_update.key);
                }
            }
        }
    }

    map.into_iter()
        .map(|(key, value)| KeyValue {
            key: key.into_owned(),
            value,
        })
        .collect()
}
