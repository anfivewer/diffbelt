use crate::collection::methods::diff::{DiffOk, DiffOptions, ReadDiffCursorOptions};
use crate::collection::methods::put::CollectionPutManyOptions;

use crate::collection::Collection;
use crate::common::generation_id::GenerationIdSource;
use crate::common::{
    GenerationId, IsByteArrayMut, KeyValueDiff, KeyValueUpdate, KeyValueUpdateNewOptions,
    OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId,
};
use crate::database::config::DatabaseConfig;
use crate::database::create_collection::CreateCollectionOptions;
use crate::tests::temp_database::TempDatabase;
use crate::tests::util::manual_generation::wrap_generation;
use crate::util::bytes::{from_u32_be, increment};
use crate::util::tokio_runtime::create_main_tokio_runtime;
use std::collections::{HashMap, HashSet};

const PACK_LIMIT: usize = 20;

#[test]
fn diff_test() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(diff_test_inner());
}

async fn diff_test_inner() {
    let temp_database = TempDatabase::new_with_config(DatabaseConfig {
        diff_changes_limit: 400,
        diff_pack_limit: PACK_LIMIT,
        diff_pack_records_limit: 50,
        ..Default::default()
    })
    .await;

    let database = temp_database.get_database();

    let collection = database
        .create_collection("manual", CreateCollectionOptions { is_manual: true })
        .await
        .unwrap();

    let mut last_generation_id_bytes = [0u8];
    let first_generation_id =
        OwnedGenerationId::from_boxed_slice(last_generation_id_bytes.into()).unwrap();

    let mut first_generation_items = HashMap::with_capacity(1024);

    // Check 500 items in first gen, it will use single-generation mode and will check pack limit
    wrap_generation(&collection, first_generation_id.as_ref(), async {
        let mut items = Vec::with_capacity(500);

        for i in 0..500 {
            let key = OwnedCollectionKey::from_boxed_slice(from_u32_be(i).into()).unwrap();
            let value = OwnedCollectionValue::new(&from_u32_be(i + 1));

            first_generation_items.insert(key.clone(), value.clone());

            items.push(KeyValueUpdate::new(KeyValueUpdateNewOptions {
                key,
                value: Some(value),
                if_not_present: false,
            }));
        }

        collection
            .put_many(CollectionPutManyOptions {
                items,
                generation_id: Some(first_generation_id.clone()),
                phantom_id: None,
            })
            .await
            .unwrap();
    })
    .await;

    let expected_diff = make_diff(&HashMap::with_capacity(0), &first_generation_items);

    assert_diff(
        &collection,
        AssertDiffMode::Start(AssertDiffStart {
            from_generation_id: None,
            to_generation_id_loose: None,
        }),
        first_generation_id.as_ref(),
        &expected_diff,
        Some(&[20usize; 25]),
    )
    .await;

    let mut last_generation_items = first_generation_items.clone();

    // Change 20 records in 15 generations, it's 300 changes and also will trigger pack records limit
    {
        let mut items = Vec::with_capacity(20);
        let mut generation_id = first_generation_id.clone();

        for generation_index in 0..15 {
            items.clear();

            {
                increment(&mut last_generation_id_bytes); // for sync
                let bytes = generation_id.get_byte_array_mut();
                increment(bytes);
            }

            wrap_generation(&collection, generation_id.as_ref(), async {
                for i in 0..20 {
                    let key = OwnedCollectionKey::from_boxed_slice(from_u32_be(i).into()).unwrap();
                    let value = OwnedCollectionValue::new(&from_u32_be(i + 2 + generation_index));

                    last_generation_items.insert(key.clone(), value.clone());

                    items.push(KeyValueUpdate::new(KeyValueUpdateNewOptions {
                        key,
                        value: Some(value),
                        if_not_present: false,
                    }));
                }

                collection
                    .put_many(CollectionPutManyOptions {
                        items: items.clone(),
                        generation_id: Some(generation_id.clone()),
                        phantom_id: None,
                    })
                    .await
                    .unwrap();
            })
            .await;
        }
    }

    let generation16_id =
        OwnedGenerationId::from_boxed_slice(last_generation_id_bytes.into()).unwrap();

    let mut expected_pack_size_distribution = [20usize; 31];
    expected_pack_size_distribution[0] = 2;
    for i in 1..6 {
        expected_pack_size_distribution[i] = 3;
    }
    expected_pack_size_distribution[6] = 4;
    expected_pack_size_distribution[30] = 19;

    // should stuck on first generation, since it has 500 changes that is bigger than `diff_changes_limit`
    assert_diff(
        &collection,
        AssertDiffMode::Start(AssertDiffStart {
            from_generation_id: None,
            to_generation_id_loose: None,
        }),
        first_generation_id.as_ref(),
        &expected_diff,
        Some(&expected_pack_size_distribution),
    )
    .await;

    let expected_diff = make_diff(&first_generation_items, &last_generation_items);

    let mut expected_pack_size_distribution = [3usize; 7];
    expected_pack_size_distribution[0] = 2;

    assert_diff(
        &collection,
        AssertDiffMode::Start(AssertDiffStart {
            from_generation_id: Some(first_generation_id.as_ref()),
            to_generation_id_loose: None,
        }),
        generation16_id.as_ref(),
        &expected_diff,
        Some(&expected_pack_size_distribution),
    )
    .await;
}

fn make_diff(
    items_from_generation_id: &HashMap<OwnedCollectionKey, OwnedCollectionValue>,
    items_to_generation_id: &HashMap<OwnedCollectionKey, OwnedCollectionValue>,
) -> Vec<KeyValueDiff> {
    let capacity = Ord::max(items_from_generation_id.len(), items_to_generation_id.len());
    let mut result = Vec::with_capacity(capacity);

    let mut all_keys = HashSet::with_capacity(capacity);

    for key in items_from_generation_id.keys() {
        all_keys.insert(key);
    }
    for key in items_to_generation_id.keys() {
        all_keys.insert(key);
    }

    let mut all_keys: Vec<&OwnedCollectionKey> = all_keys.into_iter().collect();

    all_keys.sort();

    for key in all_keys {
        let from_value = items_from_generation_id.get(key);
        let to_value = items_to_generation_id.get(key);

        if from_value == to_value {
            continue;
        }

        result.push(KeyValueDiff {
            key: key.clone(),
            from_value: from_value.map(|x| x.clone()),
            intermediate_values: vec![],
            to_value: to_value.map(|x| x.clone()),
        });
    }

    result
}

struct AssertDiffStart<'a> {
    from_generation_id: Option<GenerationId<'a>>,
    to_generation_id_loose: Option<GenerationId<'a>>,
}

enum AssertDiffMode<'a> {
    Start(AssertDiffStart<'a>),
    ByCursor(Box<str>),
}

async fn assert_diff(
    collection: &Collection,
    diff_mode: AssertDiffMode<'_>,
    expected_to_generation_id: GenerationId<'_>,
    expected_diff: &[KeyValueDiff],
    expected_pack_size_distribution: Option<&[usize]>,
) {
    let mut diff_mode = diff_mode;
    let mut expected_diff = expected_diff;
    let mut expected_pack_size_distribution = expected_pack_size_distribution;

    loop {
        let result = match diff_mode {
            AssertDiffMode::Start(AssertDiffStart {
                from_generation_id,
                to_generation_id_loose,
            }) => collection
                .diff(DiffOptions {
                    from_generation_id: GenerationIdSource::Value(
                        from_generation_id.map(|id| id.to_owned()),
                    ),
                    to_generation_id_loose: to_generation_id_loose.map(|id| id.to_owned()),
                })
                .await
                .unwrap(),
            AssertDiffMode::ByCursor(cursor_id) => collection
                .read_diff_cursor(ReadDiffCursorOptions { cursor_id })
                .await
                .unwrap(),
        };

        let DiffOk {
            from_generation_id: _,
            to_generation_id,
            items,
            cursor_id,
        } = result;

        assert_eq!(to_generation_id.as_ref(), expected_to_generation_id);

        let expected_items_count = expected_diff.len();
        let items_count = items.len();

        assert!(items_count <= expected_items_count);
        assert!(
            items_count <= PACK_LIMIT,
            "items_count = {}, PACK_LIMIT = {}",
            items_count,
            PACK_LIMIT
        );

        if items_count == 0 && expected_items_count == 0 {
            assert!(cursor_id.is_none());
            return;
        }

        if let Some(expected) = expected_pack_size_distribution {
            assert_eq!(items_count, expected[0]);
        }

        let mut items_iterator = items.into_iter();
        let mut expected_items_iterator = expected_diff.iter();
        let mut index = 0usize;

        loop {
            let item = items_iterator.next();

            let item = match item {
                Some(item) => item,
                None => {
                    break;
                }
            };

            let expected_item = expected_items_iterator.next().unwrap();
            index += 1;

            assert_eq!(
                &item, expected_item,
                "toGenerationId = {:?}",
                to_generation_id
            );
        }

        expected_diff = &expected_diff[index..];
        expected_pack_size_distribution =
            expected_pack_size_distribution.map(|expected| &expected[1..]);

        let cursor_id = match cursor_id {
            Some(id) => id,
            None => {
                assert_eq!(expected_diff.len(), 0);
                assert_eq!(
                    expected_pack_size_distribution
                        .map(|expected| expected.len())
                        .unwrap_or(0),
                    0
                );
                return;
            }
        };

        diff_mode = AssertDiffMode::ByCursor(cursor_id);
    }
}
