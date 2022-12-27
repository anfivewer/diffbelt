use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::query::{QueryOk, QueryOptions, ReadQueryCursorOptions};
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::collection::Collection;
use crate::common::{
    GenerationId, KeyValue, KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue,
    OwnedGenerationId,
};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::util::global_tokio_runtime::create_global_tokio_runtime;
use std::collections::BTreeMap;

use futures::future::BoxFuture;

#[test]
fn query_test() {
    let runtime = create_global_tokio_runtime().unwrap();
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

        first_generation_updates.push(KeyValueUpdate {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: Some(OwnedCollectionValue::new(&value)),
            if_not_present: false,
        });
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
        })
        .await
        .unwrap();

    let first_generation_expected_items =
        key_value_update_items_to_key_value(first_generation_updates.clone());
    assert_query(
        collection.as_ref(),
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

        second_generation_updates.push(KeyValueUpdate {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: Some(OwnedCollectionValue::new(&value)),
            if_not_present: false,
        });
    }

    // 20 items we will remove
    for i in 30..50 {
        let key = [(i % 256) as u8];

        second_generation_updates.push(KeyValueUpdate {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: None,
            if_not_present: false,
        });
    }

    // 250 items will be added
    for i in 100..350 {
        let key = [(i % 256) as u8, (i % 29) as u8];
        let value = [(i % 2) as u8, (i % 3) as u8, (i % 4) as u8, (i % 5) as u8];

        second_generation_updates.push(KeyValueUpdate {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: Some(OwnedCollectionValue::new(&value)),
            if_not_present: false,
        });
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
        })
        .await
        .unwrap();

    // First generation should not be changed
    assert_query(
        collection.as_ref(),
        Some(first_generation_id.clone()),
        first_generation_id.as_ref(),
        &first_generation_expected_items,
    )
    .await;

    let second_generation_expected_items =
        merge_kv_updates(vec![&first_generation_updates, &second_generation_updates]);

    assert_query(
        collection.as_ref(),
        None,
        second_generation_id.as_ref(),
        &second_generation_expected_items,
    )
    .await;
}

fn key_value_update_items_to_key_value(items: Vec<KeyValueUpdate>) -> Vec<KeyValue> {
    let mut items: Vec<KeyValue> = items
        .into_iter()
        .filter_map(|key_value_update| {
            key_value_update.value.map(|value| KeyValue {
                key: key_value_update.key,
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
        .map(|(key, value)| KeyValue { key, value })
        .collect()
}

async fn assert_query(
    collection: &Collection,
    quering_generation_id: Option<OwnedGenerationId>,
    expected_generation_id: GenerationId<'_>,
    expected_items: &[KeyValue],
) {
    assert_query_inner(
        collection,
        quering_generation_id,
        expected_generation_id,
        expected_items,
        None,
    )
    .await
}

fn assert_query_inner<'a>(
    collection: &'a Collection,
    quering_generation_id: Option<OwnedGenerationId>,
    expected_generation_id: GenerationId<'a>,
    expected_items: &'a [KeyValue],
    cursor_id: Option<String>,
) -> BoxFuture<'a, ()> {
    let fut = async move {
        let result = match cursor_id {
            Some(cursor_id) => collection
                .read_query_cursor(ReadQueryCursorOptions { cursor_id })
                .await
                .unwrap(),
            None => collection
                .query(QueryOptions {
                    generation_id: quering_generation_id.clone(),
                    phantom_id: None,
                })
                .await
                .unwrap(),
        };

        let QueryOk {
            generation_id: actual_generation_id,
            items: actual_items,
            cursor_id,
        } = result;

        assert_eq!(actual_generation_id.as_ref(), expected_generation_id);

        // WARN: when custom limits on packs will be implemented, it can fail
        assert!(actual_items.len() <= 200);

        let this_pack_expected_items = &expected_items[0..(actual_items.len())];
        let next_pack_expected_items = &expected_items[(actual_items.len())..];

        assert_eq!(&actual_items, this_pack_expected_items);

        assert_eq!(cursor_id.is_none(), next_pack_expected_items.is_empty());

        if next_pack_expected_items.is_empty() {
            return ();
        }

        assert_query_inner(
            collection,
            quering_generation_id,
            expected_generation_id,
            next_pack_expected_items,
            cursor_id,
        )
        .await
    };

    Box::pin(fut)
}
