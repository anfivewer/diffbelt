use std::cmp::max;
use std::collections::HashMap;
use std::mem;
use std::str::from_utf8;

use lazy_static::lazy_static;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use regex::Regex;

use diffbelt_protos::protos::transform::aggregate::{
    AggregateMapMultiOutput, AggregateMapMultiOutputArgs, AggregateMapOutput,
    AggregateMapOutputArgs,
};
use diffbelt_protos::Serializer;
use diffbelt_types::collection::diff::{
    DiffCollectionRequestJsonData, DiffCollectionResponseJsonData, KeyValueDiffJsonData,
    ReaderDiffFromDefJsonData,
};
use diffbelt_types::collection::generation::StartGenerationRequestJsonData;
use diffbelt_types::collection::get_record::{GetRequestJsonData, GetResponseJsonData};
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_types::common::key_value::{
    EncodedKeyJsonData, EncodedValueJsonData, KeyValueJsonData,
};
use diffbelt_util_no_std::cast::{u32_to_i64, u32_to_u64, u32_to_usize};

use crate::aggregate::AggregateTransform;
use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::function_eval::{
    AggregateInitialAccumulatorEvalAction, AggregateMapEvalAction, AggregateMergeEvalAction,
    AggregateReduceEvalAction, AggregateTargetInfoEvalAction, FunctionEvalAction,
};
use crate::base::action::{Action, ActionType};
use crate::base::common::accumulator::AccumulatorId;
use crate::base::common::target_info::TargetInfoId;
use crate::base::input::diffbelt_call::{DiffbeltCallInput, DiffbeltResponseBody};
use crate::base::input::function_eval::{
    AggregateInitialAccumulatorEvalInput, AggregateMapEvalInput, AggregateMergeEvalInput,
    AggregateReduceEvalInput, AggregateTargetInfoEvalInput, FunctionEvalInput,
    FunctionEvalInputBody,
};
use crate::base::input::{Input, InputType};
use crate::TransformRunResult;

#[test]
fn aggregate_test() {
    run_aggregate_test(AggregateTestParams {
        source_items_count: 1000,
        new_items_count: 500,
        modify_items_count: 300,
        delete_items_count: 200,
        target_buckets_count: 20,
        rand: ChaCha8Rng::seed_from_u64(0x9a9ddd206ce854ef),
    });
}

struct AggregateTestParams<Random: Rng> {
    source_items_count: usize,
    new_items_count: usize,
    modify_items_count: usize,
    delete_items_count: usize,
    target_buckets_count: usize,
    rand: Random,
}

fn run_aggregate_test<Random: Rng>(params: AggregateTestParams<Random>) {
    let AggregateTestParams {
        source_items_count,
        new_items_count,
        modify_items_count,
        delete_items_count,
        target_buckets_count,
        mut rand,
    } = params;

    let mut source_items = Vec::with_capacity(source_items_count);
    let target_items = target_items_from_source(&source_items);
    let initial_target_items = target_items.clone();

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

    fn take_items<Rand: Rng, ItemsIter: Iterator<Item = KeyValueDiffJsonData>>(
        rand: &mut Rand,
        diff_items_left: &mut usize,
        diff_items_iter: &mut ItemsIter,
        diff_cursor_counter: &mut usize,
        diff_cursor: &mut Option<String>,
    ) -> Vec<KeyValueDiffJsonData> {
        let diff_items_to_take = rand.gen_range(0..(*diff_items_left + 1));
        *diff_items_left -= diff_items_to_take;
        let items: Vec<_> = diff_items_iter.take(diff_items_to_take).collect();

        *diff_cursor_counter += 1;
        *diff_cursor = Some(format!("cursor{diff_cursor_counter}"));

        items
    }

    let mut target_info_counter = 0u64;
    let mut accumulator_counter = 0u64;
    let mut target_infos = HashMap::new();

    struct AccumulatorData {
        target_info: TargetInfoId,
        diff: i64,
    }
    let mut accumulators = HashMap::new();

    let new_target_items = target_items_from_source(&new_source_items);

    let mut transform = AggregateTransform::new(
        Box::from("source"),
        Box::from("target"),
        Box::from("reader"),
        true,
    );

    let mut inputs = Vec::new();

    let mut pending_actions = Vec::new();

    loop {
        let mut old_inputs = Vec::new();
        mem::swap(&mut inputs, &mut old_inputs);

        let run_result = transform.run(old_inputs).expect("should run");

        let mut actions = match run_result {
            TransformRunResult::Actions(actions) => actions,
            TransformRunResult::Finish => {
                break;
            }
        };

        pending_actions.extend(actions.drain(..));

        transform.return_actions_vec(actions);
        
        if pending_actions.is_empty() {
            transform.debug_print();
            
            panic!("no more actions");
        }

        let actions_to_process_count = rand.gen_range(1..(pending_actions.len() + 1));

        for _ in 0..actions_to_process_count {
            let index = rand.gen_range(0..pending_actions.len());
            let action = pending_actions.swap_remove(index);

            let Action {
                id: action_id,
                action,
            } = action;

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

                        let items = take_items(
                            &mut rand,
                            &mut diff_items_left,
                            &mut diff_items_iter,
                            &mut diff_cursor_counter,
                            &mut diff_cursor,
                        );

                        inputs.push(Input {
                            id: action_id,
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
                            id: action_id,
                            input: InputType::DiffbeltCall(DiffbeltCallInput {
                                body: DiffbeltResponseBody::Ok(()),
                            }),
                        });

                        continue;
                    }

                    lazy_static! {
                        static ref DIFF_RE: Regex =
                            Regex::new("^/collections/source/diff/([^/]+)$").unwrap();
                    }

                    let diff_captures = DIFF_RE.captures(&path);

                    if let Some(m) = diff_captures.and_then(|x| x.get(1)) {
                        let is_current_cursor = diff_cursor
                            .as_ref()
                            .map(|x| x.as_str() == m.as_str())
                            .unwrap_or(false);

                        if is_current_cursor {
                            assert_eq!(method, Method::Get);
                            assert!(query.is_empty());
                            assert_eq!(body, DiffbeltRequestBody::ReadDiffCursorNone,);

                            let items = take_items(
                                &mut rand,
                                &mut diff_items_left,
                                &mut diff_items_iter,
                                &mut diff_cursor_counter,
                                &mut diff_cursor,
                            );

                            inputs.push(Input {
                                id: action_id,
                                input: InputType::diffbelt_call(DiffbeltResponseBody::Diff(
                                    DiffCollectionResponseJsonData {
                                        from_generation_id: EncodedGenerationIdJsonData::new_str(
                                            "first".to_string(),
                                        ),
                                        to_generation_id: EncodedGenerationIdJsonData::new_str(
                                            "second".to_string(),
                                        ),
                                        items,
                                        cursor_id: if diff_items_left == 0 {
                                            None
                                        } else {
                                            diff_cursor.as_ref().map(|x| Box::from(x.as_str()))
                                        },
                                    },
                                )),
                            });

                            continue;
                        }
                    }

                    if &path == "/collections/target/get" {
                        assert_eq!(method, Method::Post);
                        assert!(query.is_empty());

                        let body = body.into_get_record().expect("should be get body");
                        let GetRequestJsonData {
                            key,
                            generation_id,
                            phantom_id,
                        } = body;

                        assert!(phantom_id.is_none());

                        let generation_id = generation_id
                            .expect("get record request should have generation")
                            .into_bytes()
                            .expect("parse bytes");

                        assert_eq!(generation_id.as_ref(), "second".as_bytes());

                        let key = key.into_bytes().expect("parse bytes");

                        let key_string = String::from_utf8(key.into_vec()).expect("should be utf8");

                        let key = key_string.parse::<u32>().expect("should be number");

                        let value = target_items.get(&key);
                        let item = value.map(|value| KeyValueJsonData {
                            key: EncodedKeyJsonData::new_str(key_string),
                            value: EncodedValueJsonData::new_str(value.to_string()),
                        });

                        inputs.push(Input {
                            id: action_id,
                            input: InputType::DiffbeltCall(DiffbeltCallInput {
                                body: DiffbeltResponseBody::GetRecord(GetResponseJsonData {
                                    generation_id: EncodedGenerationIdJsonData::new_str(
                                        String::from("second"),
                                    ),
                                    item,
                                }),
                            }),
                        });

                        continue;
                    }

                    panic!("unexpected diffbelt call {method:?} {path} {query:?} {body:?}");
                }
                ActionType::FunctionEval(call) => {
                    match call {
                        FunctionEvalAction::AggregateMap(map) => {
                            let AggregateMapEvalAction {
                                input,
                                output_buffer,
                            } = map;

                            let map_multi_input = input.data();
                            let items = map_multi_input.items().unwrap_or_default();

                            let mut serializer = Serializer::from_vec(output_buffer);
                            let mut records = Vec::with_capacity(items.len());

                            for item in items {
                                let source_key = item.source_key().expect("key should be present");
                                let source_old_value = item.source_old_value();
                                let source_new_value = item.source_new_value();

                                let source_key = String::from_utf8(source_key.bytes().to_owned())
                                    .expect("should be utf8")
                                    .parse::<u32>()
                                    .expect("should be number");
                                let source_old_value = source_old_value.map(|x| {
                                    String::from_utf8(x.bytes().to_owned())
                                        .expect("should be utf8")
                                        .parse::<u32>()
                                        .expect("should be number")
                                });
                                let source_new_value = source_new_value.map(|x| {
                                    String::from_utf8(x.bytes().to_owned())
                                        .expect("should be utf8")
                                        .parse::<u32>()
                                        .expect("should be number")
                                });

                                let source_old_value =
                                    source_old_value.map(|x| u32_to_i64(x)).unwrap_or(0);
                                let old_mapped_value = -source_old_value;
                                let old_mapped_value = old_mapped_value.to_string();
                                let new_mapped_value = source_new_value
                                    .map(|x| u32_to_i64(x))
                                    .unwrap_or(0)
                                    .to_string();

                                let target_key = u32_to_usize(source_key) % target_buckets_count;
                                let target_key = target_key.to_string();
                                let target_key = serializer.create_vector(target_key.as_bytes());
                                let old_mapped_value =
                                    serializer.create_vector(old_mapped_value.as_bytes());
                                let new_mapped_value =
                                    serializer.create_vector(new_mapped_value.as_bytes());

                                records.push(AggregateMapOutput::create(
                                    serializer.buffer_builder(),
                                    &AggregateMapOutputArgs {
                                        target_key: Some(target_key),
                                        mapped_value: Some(old_mapped_value),
                                    },
                                ));
                                records.push(AggregateMapOutput::create(
                                    serializer.buffer_builder(),
                                    &AggregateMapOutputArgs {
                                        target_key: Some(target_key),
                                        mapped_value: Some(new_mapped_value),
                                    },
                                ));
                            }

                            let records = serializer.create_vector(&records);

                            let result = AggregateMapMultiOutput::create(
                                serializer.buffer_builder(),
                                &AggregateMapMultiOutputArgs {
                                    items: Some(records),
                                },
                            );

                            let result = serializer.finish(result).into_owned();

                            inputs.push(Input {
                                id: action_id,
                                input: InputType::FunctionEval(FunctionEvalInput {
                                    body: FunctionEvalInputBody::AggregateMap(
                                        AggregateMapEvalInput {
                                            input: result,
                                            action_input_buffer: input.into_vec(),
                                        },
                                    ),
                                }),
                            });

                            continue;
                        }
                        FunctionEvalAction::AggregateTargetInfo(action) => {
                            let AggregateTargetInfoEvalAction { target_info } = action;

                            target_info_counter += 1;
                            let target_info_id = target_info_counter;

                            target_infos.insert(target_info_id, target_info);

                            inputs.push(Input {
                                id: action_id,
                                input: InputType::FunctionEval(FunctionEvalInput {
                                    body: FunctionEvalInputBody::AggregateTargetInfo(
                                        AggregateTargetInfoEvalInput {
                                            target_info_id: TargetInfoId(target_info_id),
                                        },
                                    ),
                                }),
                            });

                            continue;
                        }
                        FunctionEvalAction::AggregateInitialAccumulator(action) => {
                            let AggregateInitialAccumulatorEvalAction { target_info } = action;

                            accumulator_counter += 1;
                            let accumulator_id = accumulator_counter;

                            accumulators.insert(
                                accumulator_id,
                                AccumulatorData {
                                    target_info,
                                    diff: 0,
                                },
                            );

                            inputs.push(Input {
                                id: action_id,
                                input: InputType::FunctionEval(FunctionEvalInput {
                                    body: FunctionEvalInputBody::AggregateInitialAccumulator(
                                        AggregateInitialAccumulatorEvalInput {
                                            accumulator_id: AccumulatorId(accumulator_id),
                                        },
                                    ),
                                }),
                            });

                            continue;
                        }
                        FunctionEvalAction::AggregateReduce(action) => {
                            let AggregateReduceEvalAction {
                                accumulator,
                                target_info,
                                input: input_serialized,
                            } = action;

                            assert!(target_infos.contains_key(&target_info.0));

                            let accumulator_data = accumulators
                                .get_mut(&accumulator.0)
                                .expect("accumulator should exist");

                            let input = input_serialized.data();
                            let items = input.items().expect("items should not be empty");

                            for item in items {
                                let mapped_value =
                                    item.mapped_value().expect("mapped value should be present");
                                let mapped_value = mapped_value.bytes();
                                let mapped_value = from_utf8(mapped_value).expect("not utf8");
                                let diff = mapped_value
                                    .parse::<i64>()
                                    .expect("cannot parse reduce item");

                                accumulator_data.diff += diff;
                            }

                            inputs.push(Input {
                                id: action_id,
                                input: InputType::FunctionEval(FunctionEvalInput {
                                    body: FunctionEvalInputBody::AggregateReduce(
                                        AggregateReduceEvalInput {
                                            accumulator_id: accumulator,
                                            action_input_buffer: input_serialized.into_vec(),
                                        },
                                    ),
                                }),
                            });

                            continue;
                        }
                        FunctionEvalAction::AggregateMerge(action) => {
                            let AggregateMergeEvalAction {
                                target_info,
                                accumulator_ids: input,
                            } = action;

                            let mut result = AccumulatorData {
                                target_info,
                                diff: 0,
                            };

                            input.iter().fold(&mut result, |acc, x| {
                                let AccumulatorData {
                                    target_info: acc_target_info,
                                    diff,
                                } = accumulators.remove(&x.0).expect("accumulator not found");

                                assert_eq!(
                                    &acc_target_info, &acc.target_info,
                                    "different target_info"
                                );

                                acc.diff += diff;

                                acc
                            });

                            transform.return_merge_accumulator_ids_vec(input);

                            accumulator_counter += 1;
                            let accumulator_id = accumulator_counter;

                            accumulators.insert(accumulator_id, result);

                            inputs.push(Input {
                                id: action_id,
                                input: InputType::FunctionEval(FunctionEvalInput {
                                    body: FunctionEvalInputBody::AggregateMerge(
                                        AggregateMergeEvalInput {
                                            accumulator_id: AccumulatorId(accumulator_id),
                                        },
                                    ),
                                }),
                            });

                            continue;
                        }
                        _ => {}
                    }

                    panic!("unexpected function eval {:?}", call);
                }
            }
        }
    }
}

fn target_items_from_source(source_items: &[(u32, u32)]) -> HashMap<u32, u64> {
    let mut target_items = HashMap::new();

    for (key, value) in source_items {
        let target_key = *key % 1024;

        target_items
            .entry(target_key)
            .and_modify(|val| *val += u32_to_u64(*value))
            .or_insert(u32_to_u64(*value));
    }

    target_items
}
