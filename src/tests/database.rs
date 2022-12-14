use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::get::CollectionGetOptions;
use crate::collection::methods::put::{CollectionPutManyOptions, CollectionPutOptions};
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::{
    IsByteArrayMut, KeyValueUpdate, OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId,
};
use crate::config::Config;
use crate::database::create_collection::CreateCollectionOptions;
use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;
use crate::raw_db::{RawDb, RawDbOptions};
use crate::tests::temp_dir::TempDir;
use crate::util::global_tokio_runtime::create_global_tokio_runtime;

use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

#[test]
fn database_test() {
    let runtime = create_global_tokio_runtime().unwrap();
    runtime.block_on(database_test_inner());
}

async fn database_test_inner() {
    let temp_dir = TempDir::new().unwrap();

    println!("Temp dir: {:?}", temp_dir.get_path_buf());

    let config = Arc::new(Config {
        data_path: temp_dir.get_path_buf().to_str().unwrap().to_string(),
    });

    let meta_raw_db_path = temp_dir.get_path_buf().join("_meta");
    let meta_raw_db_path = meta_raw_db_path.to_str().unwrap();

    let meta_raw_db = RawDb::open_raw_db(RawDbOptions {
        path: meta_raw_db_path,
        comparator: None,
        column_families: vec![],
    })
    .expect("Cannot open meta raw_db");

    let meta_raw_db = Arc::new(meta_raw_db);

    let database = Database::open(DatabaseOpenOptions {
        config: config.clone(),
        meta_raw_db: meta_raw_db.clone(),
    })
    .await
    .expect("Cannot open database");

    let collection = database
        .get_or_create_collection("test", CreateCollectionOptions { is_manual: false })
        .await
        .expect("Collection create");

    let manual_collection = database
        .get_or_create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .expect("Collection create");

    let result = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.item.is_none());
    assert_eq!(
        result.generation_id,
        OwnedGenerationId(vec![0; 64].into_boxed_slice())
    );

    let result = collection
        .put(CollectionPutOptions {
            update: KeyValueUpdate {
                key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
                value: Option::Some(OwnedCollectionValue::new(b"passed")),
                if_not_present: true,
            },
            generation_id: None,
            phantom_id: None,
        })
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.was_put, true);
    let mut generation_id_of_put = OwnedGenerationId(vec![0; 64].into_boxed_slice());
    let generation_id_mut = generation_id_of_put.get_byte_array_mut();
    generation_id_mut[63] = 1;
    assert_eq!(result.generation_id, generation_id_of_put);

    let mut generation_id_receiver = collection.get_generation_id_receiver();

    loop {
        let is_got_it = {
            let generation_id = generation_id_receiver.borrow_and_update();
            generation_id.deref() >= &generation_id_of_put
        };

        if is_got_it {
            break;
        }

        timeout(Duration::from_millis(100), generation_id_receiver.changed())
            .await
            .unwrap()
            .unwrap();
    }

    let result = collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.item.is_some());
    let mut generation_id = OwnedGenerationId(vec![0; 64].into_boxed_slice());
    let generation_id_mut = generation_id.get_byte_array_mut();
    generation_id_mut[63] = 1;
    assert_eq!(result.generation_id, generation_id);

    let result = collection
        .put_many(CollectionPutManyOptions {
            items: vec![
                KeyValueUpdate {
                    key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
                    value: Option::Some(OwnedCollectionValue::new(b"passed3")),
                    if_not_present: true,
                },
                KeyValueUpdate {
                    key: OwnedCollectionKey(b"test2".to_vec().into_boxed_slice()),
                    value: Option::Some(OwnedCollectionValue::new(b"passed again")),
                    if_not_present: true,
                },
            ],
            generation_id: None,
            phantom_id: None,
        })
        .await;

    // TODO: assert results below

    println!("put result {:?}", result);

    let result = manual_collection
        .start_generation(StartGenerationOptions {
            generation_id: OwnedGenerationId(b"first".to_vec().into_boxed_slice()),
            abort_outdated: false,
        })
        .await;

    println!("start generation result {:?}", result);

    let result = manual_collection
        .put(CollectionPutOptions {
            update: KeyValueUpdate {
                key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
                value: Option::Some(OwnedCollectionValue::new(b"passed")),
                if_not_present: true,
            },
            generation_id: Some(OwnedGenerationId(b"first".to_vec().into_boxed_slice())),
            phantom_id: None,
        })
        .await;

    println!("put in manual result {:?}", result);

    let result = manual_collection
        .commit_generation(CommitGenerationOptions {
            generation_id: OwnedGenerationId(b"first".to_vec().into_boxed_slice()),
        })
        .await;

    println!("commit generation result {:?}", result);

    let result = manual_collection
        .get(CollectionGetOptions {
            key: OwnedCollectionKey(b"test".to_vec().into_boxed_slice()),
            generation_id: None,
            phantom_id: None,
        })
        .await;

    println!("get from manual result {:?}", result);

    // TODO: test readers
}
