use std::cmp::max;
use std::mem;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::aggregate::AggregateTransform;
use diffbelt_types::collection::diff::KeyValueDiffJsonData;
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use crate::TransformRunResult;

#[test]
fn aggregate_test() {
    run_aggregate_test(AggregateTestParams {
        source_items_count: 1000,
        new_items_count: 500,
        modify_items_count: 300,
        delete_items_count: 200,
        rand: ChaCha8Rng::seed_from_u64(0x9a9ddd206ce854ef),
    });
}

struct AggregateTestParams<Random: Rng> {
    source_items_count: usize,
    new_items_count: usize,
    modify_items_count: usize,
    delete_items_count: usize,
    rand: Random,
}

fn run_aggregate_test<Random: Rng>(params: AggregateTestParams<Random>) {
    let AggregateTestParams {
        source_items_count,
        new_items_count,
        modify_items_count,
        delete_items_count,
        mut rand,
    } = params;

    let mut source_items = Vec::with_capacity(source_items_count);

    for _ in 0..source_items_count {
        source_items.push((rand.next_u32(), rand.next_u32()));
    }

    let mut target_items = Vec::with_capacity(max(
        source_items_count + new_items_count - delete_items_count,
        source_items.len(),
    ));
    target_items.extend(source_items);

    let mut diff_items =
        Vec::with_capacity(new_items_count + modify_items_count + delete_items_count);

    for _ in 0..delete_items_count {
        let index = rand.gen_range(0..target_items.len());

        let (key, value) = target_items.swap_remove(index);

        diff_items.push(KeyValueDiffJsonData {
            key: EncodedKeyJsonData::new_str(key.to_string()),
            from_value: Some(Some(EncodedValueJsonData::new_str(value.to_string()))),
            intermediate_values: vec![],
            to_value: None,
        });
    }

    for i in 0..modify_items_count {
        let index = rand.gen_range(i..target_items.len());

        target_items.swap(i, index);

        let (key, value) = target_items
            .get_mut(i)
            .expect("delete_items_count should be > source_items_count");

        let old_value = *value;
        *value = rand.next_u32();

        diff_items.push(KeyValueDiffJsonData {
            key: EncodedKeyJsonData::new_str(key.to_string()),
            from_value: Some(Some(EncodedValueJsonData::new_str(old_value.to_string()))),
            intermediate_values: vec![],
            to_value: Some(Some(EncodedValueJsonData::new_str(value.to_string()))),
        });
    }

    for _ in 0..new_items_count {
        let key = rand.next_u32();
        let value = rand.next_u32();

        diff_items.push(KeyValueDiffJsonData {
            key: EncodedKeyJsonData::new_str(key.to_string()),
            from_value: None,
            intermediate_values: vec![],
            to_value: Some(Some(EncodedValueJsonData::new_str(value.to_string()))),
        });
    }

    let mut transform = AggregateTransform::new(
        Box::from("source"),
        Box::from("target"),
        Box::from("reader"),
    );

    let mut inputs = Vec::new();

    loop {
        let mut old_inputs = Vec::new();
        mem::swap(&mut inputs, &mut old_inputs);

        let run_result = transform.run(old_inputs).expect("should run");

        let actions = match run_result {
            TransformRunResult::Actions(actions) => actions,
            TransformRunResult::Finish => {
                break;
            }
        };

        for action in actions {
            todo!()
        }
    }
}
