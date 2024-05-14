use crate::collection::methods::get::{CollectionGetOk, CollectionGetOptions};
use crate::collection::methods::put::CollectionPutManyOptions;
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::common::{
    KeyValue, KeyValueUpdate, KeyValueUpdateNewOptions, OwnedCollectionKey, OwnedCollectionValue,
    OwnedGenerationId,
};
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::util::tokio_runtime::create_main_tokio_runtime;

#[test]
fn if_not_present_same_key_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(if_not_present_same_key_inner());
}

async fn if_not_present_same_key_inner() {
    let temp_database = TempDatabase::new().await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
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
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Some(OwnedCollectionValue::from_boxed_slice(
                        b"42".to_vec().into_boxed_slice(),
                    )),
                    if_not_present: true,
                }),
                KeyValueUpdate::new(KeyValueUpdateNewOptions {
                    key: OwnedCollectionKey::from_boxed_slice(b"1".to_vec().into_boxed_slice())
                        .unwrap(),
                    value: Some(OwnedCollectionValue::from_boxed_slice(
                        b"13".to_vec().into_boxed_slice(),
                    )),
                    if_not_present: true,
                }),
            ],
            generation_id: Some(first_generation_id.clone()),
            phantom_id: None,
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
            value: OwnedCollectionValue::from_boxed_slice(b"13".to_vec().into_boxed_slice(),),
        })
    );
}
