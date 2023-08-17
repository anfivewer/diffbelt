use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::util::tokio_runtime::create_main_tokio_runtime;

#[test]
fn delete_collection_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(delete_collection_test_inner());
}

async fn delete_collection_test_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    let fut = collection.delete_collection();

    drop(collection);

    fut.await.unwrap();
}
