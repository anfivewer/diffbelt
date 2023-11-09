use std::cmp::max;
use std::collections::HashMap;
use std::mem;
use std::ops::Deref;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::{Action, ActionType};
use crate::base::input::diffbelt_call::{DiffbeltCallInput, DiffbeltResponseBody};
use crate::base::input::{Input, InputType};
use crate::TransformRunResult;
use diffbelt_types::collection::diff::{
    DiffCollectionRequestJsonData, DiffCollectionResponseJsonData, KeyValueDiffJsonData,
    ReaderDiffFromDefJsonData,
};
use diffbelt_types::collection::generation::StartGenerationRequestJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use diffbelt_util_no_std::cast::{u32_to_u64, u32_to_usize};

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
    let target_items = target_items_from_source(&source_items);

    for _ in 0..source_items_count {
        source_items.push((rand.next_u32(), rand.next_u32()));
    }

    let mut new_source_items = Vec::with_capacity(max(
        source_items_count + new_items_count - delete_items_count,
        source_items.len(),
    ));
    new_source_items.extend(source_items);

    let mut diff_items =
        Vec::with_capacity(new_items_count + modify_items_count + delete_items_count);

    for _ in 0..delete_items_count {
        let index = rand.gen_range(0..new_source_items.len());

        let (key, value) = new_source_items.swap_remove(index);

        diff_items.push(KeyValueDiffJsonData {
            key: EncodedKeyJsonData::new_str(key.to_string()),
            from_value: Some(Some(EncodedValueJsonData::new_str(value.to_string()))),
            intermediate_values: vec![],
            to_value: None,
        });
    }

    for i in 0..modify_items_count {
        let index = rand.gen_range(i..new_source_items.len());

        new_source_items.swap(i, index);

        let (key, value) = new_source_items
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

        new_source_items.push((key, value));

        diff_items.push(KeyValueDiffJsonData {
            key: EncodedKeyJsonData::new_str(key.to_string()),
            from_value: None,
            intermediate_values: vec![],
            to_value: Some(Some(EncodedValueJsonData::new_str(value.to_string()))),
        });
    }

    let mut diff_items_left = diff_items.len();
    let mut diff_items_iter = diff_items.into_iter();
    let mut diff_cursor_counter = 0;
    let mut diff_cursor = None;

    let new_target_items = target_items_from_source(&new_source_items);

    let mut transform = AggregateTransform::new(
        Box::from("source"),
        Box::from("target"),
        Box::from("reader"),
    );

    let mut inputs = Vec::new();

    let mut pending_actions = Vec::new();

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

        pending_actions.extend(actions);

        assert!(!pending_actions.is_empty());

        let actions_to_process_count = rand.gen_range(1..(pending_actions.len() + 1));

        for _ in 0..actions_to_process_count {
            let index = rand.gen_range(0..pending_actions.len());
            let action = pending_actions.swap_remove(index);

            let Action { id, action } = action;

            match action {
                ActionType::DiffbeltCall(call) => {
                    let DiffbeltCallAction {
                        method,
                        path,
                        query,
                        body,
                    } = call;

                    if &path == "/collections/source/diff/" {
                        assert_eq!(method, Method::Post);
                        assert!(query.is_empty());
                        assert_eq!(
                            body,
                            DiffbeltRequestBody::DiffCollectionStart(
                                DiffCollectionRequestJsonData {
                                    from_generation_id: None,
                                    to_generation_id: None,
                                    from_reader: Some(ReaderDiffFromDefJsonData {
                                        reader_name: String::from("reader"),
                                        collection_name: Some(String::from("target"))
                                    }),
                                }
                            )
                        );

                        let diff_items_to_take = rand.gen_range(0..diff_items_left);
                        diff_items_left -= diff_items_to_take;
                        let items: Vec<_> =
                            (&mut diff_items_iter).take(diff_items_to_take).collect();

                        diff_cursor_counter += 1;
                        diff_cursor = Some(format!("cursor{diff_cursor_counter}"));

                        inputs.push(Input {
                            id,
                            input: InputType::DiffbeltCall(DiffbeltCallInput {
                                body: DiffbeltResponseBody::Diff(DiffCollectionResponseJsonData {
                                    from_generation_id: EncodedGenerationIdJsonData {
                                        value: "first".to_string(),
                                        encoding: None,
                                    },
                                    to_generation_id: EncodedGenerationIdJsonData {
                                        value: "second".to_string(),
                                        encoding: None,
                                    },
                                    items,
                                    cursor_id: diff_cursor.as_ref().map(|x| Box::from(x.as_str())),
                                }),
                            }),
                        });

                        continue;
                    }

                    if &path == "/collections/target/generation/start" {
                        assert_eq!(method, Method::Post);
                        assert!(query.is_empty());
                        assert_eq!(
                            body,
                            DiffbeltRequestBody::StartGeneration(StartGenerationRequestJsonData {
                                generation_id: EncodedGenerationIdJsonData::new_str(
                                    "second".to_string()
                                ),
                                abort_outdated: Some(true),
                            })
                        );

                        inputs.push(Input {
                            id,
                            input: InputType::DiffbeltCall(DiffbeltCallInput {
                                body: DiffbeltResponseBody::Ok(()),
                            }),
                        });

                        continue;
                    }

                    panic!("unexpected diffbelt call {method:?} {path} {query:?} {body:?}");
                }
                ActionType::FunctionEval(call) => {
                    panic!("unexpected function eval {:?}", call);
                }
            }
        }
    }
}

fn target_items_from_source(source_items: &[(u32, u32)]) -> Vec<(u32, u64)> {
    let mut target_items = HashMap::new();

    for (key, value) in source_items {
        let target_key = *key % 1024;

        target_items
            .entry(target_key)
            .and_modify(|val| *val += u32_to_u64(*value))
            .or_insert(u32_to_u64(*value));
    }

    let mut target_items: Vec<_> = target_items.into_iter().collect();

    target_items.sort_by(|a, b| a.0.cmp(&b.0));

    target_items
}
