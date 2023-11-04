use diffbelt_protos::protos::transform::map_filter::{
    MapFilterMultiInput, MapFilterMultiOutput, MapFilterMultiOutputArgs, RecordUpdate,
    RecordUpdateArgs,
};
use diffbelt_protos::{deserialize, OwnedSerialized, SerializedRawParts, Serializer, Vector};
use std::borrow::Cow;
use std::str::from_utf8;

use diffbelt_types::collection::diff::{
    DiffCollectionRequestJsonData, DiffCollectionResponseJsonData, KeyValueDiffJsonData,
    ReaderDiffFromDefJsonData,
};
use diffbelt_types::collection::generation::{
    CommitGenerationRequestJsonData, StartGenerationRequestJsonData,
};
use diffbelt_types::collection::put_many::{PutManyRequestJsonData, PutManyResponseJsonData};
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;
use diffbelt_types::common::key_value::{EncodedKeyJsonData, EncodedValueJsonData};
use diffbelt_types::common::key_value_update::KeyValueUpdateJsonData;
use diffbelt_types::common::reader::UpdateReaderJsonData;
use diffbelt_util::option::lift_result_from_option;

use crate::base::action::diffbelt_call::{DiffbeltCallAction, DiffbeltRequestBody, Method};
use crate::base::action::function_eval::{FunctionEvalAction, MapFilterEvalAction};
use crate::base::action::{Action, ActionType};
use crate::base::input::diffbelt_call::{DiffbeltCallInput, DiffbeltResponseBody};
use crate::base::input::function_eval::{
    FunctionEvalInput, FunctionEvalInputBody, MapFilterEvalInput,
};
use crate::base::input::{Input, InputType};
use crate::map_filter::MapFilterTransform;
use crate::TransformRunResult;

#[test]
fn map_filter_test() {
    let mut transform =
        MapFilterTransform::new(Box::from("from"), Box::from("to"), Box::from("reader"));

    let mut actions = transform
        .run(vec![])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    let action = actions.next().unwrap();
    assert_eq!(actions.next(), None);

    let Action { id, action } = action;

    assert_eq!(
        action,
        ActionType::DiffbeltCall(DiffbeltCallAction {
            method: Method::Post,
            path: Cow::Borrowed("/collections/from/diff/"),
            query: vec![],
            body: DiffbeltRequestBody::DiffCollectionStart(DiffCollectionRequestJsonData {
                from_generation_id: None,
                to_generation_id: None,
                from_reader: Some(ReaderDiffFromDefJsonData {
                    reader_name: "reader".to_string(),
                    collection_name: Some("to".to_string()),
                }),
            }),
        })
    );

    let mut actions = transform
        .run(vec![Input {
            id,
            input: InputType::DiffbeltCall(DiffbeltCallInput {
                body: DiffbeltResponseBody::Diff(DiffCollectionResponseJsonData {
                    from_generation_id: EncodedGenerationIdJsonData::new_str("10".to_string()),
                    to_generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                    items: vec![
                        KeyValueDiffJsonData {
                            key: EncodedKeyJsonData::new_str("k1".to_string()),
                            from_value: None,
                            intermediate_values: vec![],
                            to_value: Some(Some(EncodedValueJsonData::new_str("v1".to_string()))),
                        },
                        KeyValueDiffJsonData {
                            key: EncodedKeyJsonData::new_str("k2".to_string()),
                            from_value: Some(Some(EncodedValueJsonData::new_str("v2".to_string()))),
                            intermediate_values: vec![],
                            to_value: Some(Some(EncodedValueJsonData::new_str("v2-2".to_string()))),
                        },
                        KeyValueDiffJsonData {
                            key: EncodedKeyJsonData::new_str("k3".to_string()),
                            from_value: Some(Some(EncodedValueJsonData::new_str("v3".to_string()))),
                            intermediate_values: vec![],
                            to_value: None,
                        },
                    ],
                    cursor_id: Some(Box::from("first_cursor")),
                }),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    let action = actions.next().unwrap();
    assert_eq!(actions.next(), None);

    let Action { id, action } = action;

    assert_eq!(
        action,
        ActionType::DiffbeltCall(DiffbeltCallAction {
            method: Method::Post,
            path: Cow::Borrowed("/collections/to/generation/start"),
            query: vec![],
            body: DiffbeltRequestBody::StartGeneration(StartGenerationRequestJsonData {
                generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                abort_outdated: Some(true),
            }),
        })
    );

    let mut actions = transform
        .run(vec![Input {
            id,
            input: InputType::DiffbeltCall(DiffbeltCallInput {
                body: DiffbeltResponseBody::Ok(()),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    let Action {
        id: id1,
        action: action1,
    } = actions.next().unwrap();
    let Action {
        id: id2,
        action: action2,
    } = actions.next().unwrap();

    assert_eq!(actions.next(), None);

    assert_eq!(
        action1,
        ActionType::DiffbeltCall(DiffbeltCallAction {
            method: Method::Get,
            path: Cow::Borrowed("/collections/from/diff/first_cursor"),
            query: vec![],
            body: DiffbeltRequestBody::ReadDiffCursorNone,
        })
    );

    let ActionType::FunctionEval(FunctionEvalAction::MapFilter(action)) = action2 else {
        panic!("unexpected action {action2:?}");
    };

    assert_map_filter_eval_action(
        action,
        vec![
            ExpectedMapFilterActionRecord {
                source_key: "k1",
                source_old_value: None,
                source_new_value: Some("v1"),
            },
            ExpectedMapFilterActionRecord {
                source_key: "k2",
                source_old_value: Some("v2"),
                source_new_value: Some("v2-2"),
            },
            ExpectedMapFilterActionRecord {
                source_key: "k3",
                source_old_value: Some("v3"),
                source_new_value: None,
            },
        ],
    );

    let map_filter_input = make_map_filter_eval_input(vec![
        MapFilterEvalInputRecord {
            key: "k1-map",
            value: Some("v1-map"),
        },
        MapFilterEvalInputRecord {
            key: "k2-map",
            value: Some("v2-2-map"),
        },
        MapFilterEvalInputRecord {
            key: "k3-map",
            value: None,
        },
    ]);

    let mut actions = transform
        .run(vec![Input {
            id: id2,
            input: InputType::FunctionEval(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(map_filter_input),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    assert_eq!(actions.next(), None);

    let mut actions = transform
        .run(vec![Input {
            id: id1,
            input: InputType::DiffbeltCall(DiffbeltCallInput {
                body: DiffbeltResponseBody::Diff(DiffCollectionResponseJsonData {
                    from_generation_id: EncodedGenerationIdJsonData::new_str("10".to_string()),
                    to_generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                    items: vec![],
                    cursor_id: None,
                }),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    let Action {
        id: put_many_id,
        action,
    } = actions.next().unwrap();
    assert_eq!(
        action,
        ActionType::DiffbeltCall(DiffbeltCallAction {
            method: Method::Post,
            path: Cow::Borrowed("/collections/to/putMany"),
            query: vec![],
            body: DiffbeltRequestBody::PutMany(PutManyRequestJsonData {
                items: vec![
                    KeyValueUpdateJsonData {
                        key: EncodedKeyJsonData::new_str("k1-map".to_string()),
                        if_not_present: None,
                        value: Some(EncodedValueJsonData::new_str("v1-map".to_string()))
                    },
                    KeyValueUpdateJsonData {
                        key: EncodedKeyJsonData::new_str("k2-map".to_string()),
                        if_not_present: None,
                        value: Some(EncodedValueJsonData::new_str("v2-2-map".to_string()))
                    },
                    KeyValueUpdateJsonData {
                        key: EncodedKeyJsonData::new_str("k3-map".to_string()),
                        if_not_present: None,
                        value: None
                    },
                ],
                generation_id: Some(EncodedGenerationIdJsonData::new_str("42".to_string())),
                phantom_id: None,
            }),
        })
    );

    assert_eq!(actions.next(), None);

    let mut actions = transform
        .run(vec![Input {
            id: put_many_id,
            input: InputType::DiffbeltCall(DiffbeltCallInput {
                body: DiffbeltResponseBody::PutMany(PutManyResponseJsonData {
                    generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                }),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    let Action {
        id: commit_id,
        action,
    } = actions.next().unwrap();
    assert_eq!(
        action,
        ActionType::DiffbeltCall(DiffbeltCallAction {
            method: Method::Post,
            path: Cow::Borrowed("/collections/to/generation/commit"),
            query: vec![],
            body: DiffbeltRequestBody::CommitGeneration(CommitGenerationRequestJsonData {
                generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                update_readers: Some(vec![UpdateReaderJsonData {
                    reader_name: "reader".to_string(),
                    generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                }]),
            }),
        })
    );

    assert_eq!(actions.next(), None);

    let mut actions = transform
        .run(vec![Input {
            id: commit_id,
            input: InputType::DiffbeltCall(DiffbeltCallInput {
                body: DiffbeltResponseBody::Ok(()),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    let Action {
        id: second_diff,
        action,
    } = actions.next().unwrap();

    assert_eq!(actions.next(), None);

    assert_eq!(
        action,
        ActionType::DiffbeltCall(DiffbeltCallAction {
            method: Method::Post,
            path: Cow::Borrowed("/collections/from/diff/"),
            query: vec![],
            body: DiffbeltRequestBody::DiffCollectionStart(DiffCollectionRequestJsonData {
                from_generation_id: None,
                to_generation_id: None,
                from_reader: Some(ReaderDiffFromDefJsonData {
                    reader_name: "reader".to_string(),
                    collection_name: Some("to".to_string()),
                }),
            }),
        })
    );

    let result = transform
        .run(vec![Input {
            id: second_diff,
            input: InputType::DiffbeltCall(DiffbeltCallInput {
                body: DiffbeltResponseBody::Diff(DiffCollectionResponseJsonData {
                    from_generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                    to_generation_id: EncodedGenerationIdJsonData::new_str("42".to_string()),
                    items: vec![],
                    cursor_id: None,
                }),
            }),
        }])
        .unwrap();

    assert_eq!(result, TransformRunResult::Finish);
}

#[derive(Debug)]
struct ExpectedMapFilterActionRecord {
    source_key: &'static str,
    source_old_value: Option<&'static str>,
    source_new_value: Option<&'static str>,
}

fn assert_map_filter_eval_action(
    action: MapFilterEvalAction,
    expected: Vec<ExpectedMapFilterActionRecord>,
) {
    let MapFilterEvalAction {
        inputs_buffer,
        inputs_head,
        inputs_len,
        outputs_buffer: _,
    } = action;

    let bytes = &inputs_buffer[inputs_head..(inputs_head + inputs_len)];
    let map_filter_multi_input = deserialize::<MapFilterMultiInput>(bytes).expect("test");
    let records = map_filter_multi_input.items().expect("test");

    let mut records_iter = records.into_iter();
    let mut expected_iter = expected.into_iter();

    loop {
        let actual = records_iter.next();
        let expected = expected_iter.next();

        if let (None, None) = (&actual, &expected) {
            break;
        }

        let Some(actual) = actual else {
            panic!("No more actual records, but expected {expected:?}");
        };
        let Some(expected) = expected else {
            panic!("No more expected records, but got:\n{}", actual);
        };

        fn compare(key: &'static str, actual: Option<Vector<u8>>, expected: Option<&'static str>) {
            let actual = actual.map(|x| x.bytes()).map(|x| from_utf8(x));
            let actual = lift_result_from_option(actual).expect("test");

            if actual == expected {
                return;
            }

            panic!("Comparing {key}:\n  actual: {actual:?}\n  expected: {expected:?}");
        }

        compare("source_key", actual.source_key(), Some(expected.source_key));
        compare(
            "source_old_value",
            actual.source_old_value(),
            expected.source_old_value,
        );
        compare(
            "source_new_value",
            actual.source_new_value(),
            expected.source_new_value,
        );
    }
}

struct MapFilterEvalInputRecord {
    key: &'static str,
    value: Option<&'static str>,
}

fn make_map_filter_eval_input(records: Vec<MapFilterEvalInputRecord>) -> MapFilterEvalInput {
    let mut serializer = Serializer::<MapFilterMultiOutput>::new();

    let records = records
        .into_iter()
        .map(|record| {
            let MapFilterEvalInputRecord { key, value } = record;

            let key = serializer.create_vector(key.as_bytes());
            let value = value.map(|x| serializer.create_vector(x.as_bytes()));

            let update_record = RecordUpdate::create(
                serializer.buffer_builder(),
                &RecordUpdateArgs {
                    key: Some(key),
                    value,
                },
            );

            update_record
        })
        .collect::<Vec<_>>();

    let records = serializer.create_vector(&records);

    let multi_output = MapFilterMultiOutput::create(
        serializer.buffer_builder(),
        &MapFilterMultiOutputArgs {
            target_update_records: Some(records),
        },
    );

    let multi_output = serializer.finish(multi_output).into_owned();
    let SerializedRawParts { buffer, head, len } = multi_output.into_raw_parts();

    MapFilterEvalInput {
        inputs_buffer: buffer,
        inputs_head: head,
        inputs_len: len,
        outputs_buffer: vec![],
    }
}
