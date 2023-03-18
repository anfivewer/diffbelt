use crate::common::OwnedGenerationId;

use crate::database::create_collection::CreateCollectionOptions;

use crate::util::tokio_runtime::create_main_tokio_runtime;

use crate::collection::methods::create_reader::CreateReaderOptions;
use crate::collection::methods::update_reader::UpdateReaderOptions;

use crate::common::reader::ReaderRecord;
use crate::tests::temp_database::TempDatabase;

#[test]
fn readers_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(readers_test_inner());
}

async fn readers_test_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("test", CreateCollectionOptions { is_manual: true })
        .await
        .expect("Collection create");

    let readers = collection.list_readers().await.unwrap();
    assert_eq!(readers.items.len(), 0);

    let result = collection
        .update_reader(UpdateReaderOptions {
            reader_name: "not_exists".to_string(),
            generation_id: Some(
                OwnedGenerationId::from_boxed_slice(b"some_gen".to_vec().into_boxed_slice())
                    .unwrap(),
            ),
        })
        .await;
    assert!(result.is_err());

    let readers = collection.list_readers().await.unwrap();
    assert_eq!(readers.items.len(), 0);

    let result = collection
        .create_reader(CreateReaderOptions {
            reader_name: "first".to_string(),
            collection_name: Some("other_collection".to_string()),
            generation_id: None,
        })
        .await;
    assert!(result.is_ok());

    let result = collection
        .create_reader(CreateReaderOptions {
            reader_name: "second".to_string(),
            collection_name: None,
            generation_id: None,
        })
        .await;
    assert!(result.is_ok());

    let readers = collection.list_readers().await.unwrap();
    let mut items = readers.items;
    assert_eq!(items.len(), 2);

    items.sort_by(|a, b| a.reader_name.cmp(&b.reader_name));

    let expected_items = vec![
        ReaderRecord {
            reader_name: "first".to_string(),
            collection_name: Some("other_collection".to_string()),
            generation_id: None,
        },
        ReaderRecord {
            reader_name: "second".to_string(),
            collection_name: None,
            generation_id: None,
        },
    ];

    assert_eq!(items, expected_items);

    let result = collection
        .update_reader(UpdateReaderOptions {
            reader_name: "first".to_string(),
            generation_id: Some(
                OwnedGenerationId::from_boxed_slice(b"some_gen".to_vec().into_boxed_slice())
                    .unwrap(),
            ),
        })
        .await;
    assert!(result.is_ok());

    let result = collection
        .update_reader(UpdateReaderOptions {
            reader_name: "second".to_string(),
            generation_id: Some(
                OwnedGenerationId::from_boxed_slice(b"another_gen".to_vec().into_boxed_slice())
                    .unwrap(),
            ),
        })
        .await;
    assert!(result.is_ok());

    let readers = collection.list_readers().await.unwrap();
    let mut items = readers.items;
    assert_eq!(items.len(), 2);

    items.sort_by(|a, b| a.reader_name.cmp(&b.reader_name));

    let expected_items = vec![
        ReaderRecord {
            reader_name: "first".to_string(),
            collection_name: Some("other_collection".to_string()),
            generation_id: Some(
                OwnedGenerationId::from_boxed_slice(b"some_gen".to_vec().into_boxed_slice())
                    .unwrap(),
            ),
        },
        ReaderRecord {
            reader_name: "second".to_string(),
            collection_name: None,
            generation_id: Some(
                OwnedGenerationId::from_boxed_slice(b"another_gen".to_vec().into_boxed_slice())
                    .unwrap(),
            ),
        },
    ];

    assert_eq!(items, expected_items);
}
