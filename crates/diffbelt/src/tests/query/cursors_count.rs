use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::query::QueryOptions;
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::collection::Collection;
use crate::common::{
    KeyValueUpdate, KeyValueUpdateNewOptions, OwnedCollectionKey, OwnedCollectionValue,
    OwnedGenerationId,
};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::util::tokio_runtime::create_main_tokio_runtime;
use std::num::NonZeroUsize;

use crate::database::config::DatabaseConfig;

#[test]
fn cursors_count_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(cursors_count_test_inner());
}

async fn cursors_count_test_inner() {
    let temp_database = TempDatabase::new_with_config(DatabaseConfig {
        query_pack_limit: 10,
        max_cursors_per_collection: NonZeroUsize::new(4).unwrap(),
        ..Default::default()
    })
    .await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    initialize(&collection).await;

    let query_cursors_count = collection.query_cursors_count().await;
    assert_eq!(query_cursors_count, 0);

    for _ in 0..3 {
        let query_result = collection
            .query(QueryOptions {
                generation_id: None,
                phantom_id: None,
            })
            .await
            .unwrap();

        assert!(query_result.cursor_id.is_some());
        assert_eq!(query_result.items.len(), 10);
    }

    let query_cursors_count = collection.query_cursors_count().await;
    assert_eq!(query_cursors_count, 3);

    for _ in 0..100 {
        let query_result = collection
            .query(QueryOptions {
                generation_id: None,
                phantom_id: None,
            })
            .await
            .unwrap();

        assert!(query_result.cursor_id.is_some());
        assert_eq!(query_result.items.len(), 10);

        let query_cursors_count = collection.query_cursors_count().await;
        assert_eq!(query_cursors_count, 4);
    }
}

async fn initialize(collection: &Collection) {
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

    for i in 0..256 {
        let key = [i as u8];
        let value = [];

        first_generation_updates.push(KeyValueUpdate::new(KeyValueUpdateNewOptions {
            key: OwnedCollectionKey::from_boxed_slice((&key as &[u8]).into()).unwrap(),
            value: Some(OwnedCollectionValue::new(&value)),
            if_not_present: false,
        }));
    }

    collection
        .put_many(CollectionPutManyOptions {
            items: first_generation_updates.clone(),
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
}
