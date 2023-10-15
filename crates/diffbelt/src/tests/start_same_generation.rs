use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::OwnedGenerationId;
use crate::database::create_collection::CreateCollectionOptions;

use crate::tests::temp_database::TempDatabase;
use diffbelt_util::tokio_runtime::create_main_tokio_runtime;

#[test]
fn start_same_generation_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(start_same_generation_test_inner());
}

async fn start_same_generation_test_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    let first_generation_id =
        OwnedGenerationId::from_boxed_slice(b"0001".to_vec().into_boxed_slice()).unwrap();

    () = collection
        .start_generation(StartGenerationOptions {
            generation_id: first_generation_id.clone(),
            abort_outdated: false,
        })
        .await
        .unwrap();

    () = collection
        .commit_generation(CommitGenerationOptions {
            generation_id: first_generation_id.clone(),
            update_readers: None,
        })
        .await
        .unwrap();

    let res = collection
        .start_generation(StartGenerationOptions {
            generation_id: first_generation_id.clone(),
            abort_outdated: false,
        })
        .await;

    assert!(res.is_err());

    let Err(CollectionMethodError::OutdatedGeneration) = res else {
        panic!("expected OutdatedGeneration error");
    };

    let res = collection
        .start_generation(StartGenerationOptions {
            generation_id: first_generation_id.clone(),
            abort_outdated: true,
        })
        .await;

    assert!(res.is_err());

    let Err(CollectionMethodError::OutdatedGeneration) = res else {
        panic!("expected OutdatedGeneration error");
    };
}
