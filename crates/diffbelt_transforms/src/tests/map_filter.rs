/*
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
use std::borrow::Cow;

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
    let Action {
        id: id3,
        action: action3,
    } = actions.next().unwrap();
    let Action {
        id: id4,
        action: action4,
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

    assert_eq!(
        action2,
        ActionType::FunctionEval(FunctionEvalAction::MapFilter(MapFilterEvalAction {
            source_key: Box::from("k1".as_bytes()),
            source_old_value: None,
            source_new_value: Some(Box::from("v1".as_bytes())),
        }))
    );

    assert_eq!(
        action3,
        ActionType::FunctionEval(FunctionEvalAction::MapFilter(MapFilterEvalAction {
            source_key: Box::from("k2".as_bytes()),
            source_old_value: Some(Box::from("v2".as_bytes())),
            source_new_value: Some(Box::from("v2-2".as_bytes())),
        }))
    );

    assert_eq!(
        action4,
        ActionType::FunctionEval(FunctionEvalAction::MapFilter(MapFilterEvalAction {
            source_key: Box::from("k3".as_bytes()),
            source_old_value: Some(Box::from("v3".as_bytes())),
            source_new_value: None,
        }))
    );

    let mut actions = transform
        .run(vec![Input {
            id: id2,
            input: InputType::FunctionEval(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    old_key: None,
                    new_key: Some(Box::from("k1-map".as_bytes())),
                    value: Some(Box::from("v1-map".as_bytes())),
                }),
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
                    items: vec![KeyValueDiffJsonData {
                        key: EncodedKeyJsonData::new_str("k4".to_string()),
                        from_value: Some(Some(EncodedValueJsonData::new_str("v4".to_string()))),
                        intermediate_values: vec![],
                        to_value: Some(Some(EncodedValueJsonData::new_str("v4-2".to_string()))),
                    }],
                    cursor_id: Some(Box::from("second_cursor")),
                }),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    let Action {
        id: second_cursor_id,
        action,
    } = actions.next().unwrap();
    assert_eq!(
        action,
        ActionType::DiffbeltCall(DiffbeltCallAction {
            method: Method::Get,
            path: Cow::Borrowed("/collections/from/diff/second_cursor"),
            query: vec![],
            body: DiffbeltRequestBody::ReadDiffCursorNone,
        })
    );

    let Action {
        id: id6,
        action: action6,
    } = actions.next().unwrap();

    assert_eq!(actions.next(), None);

    assert_eq!(
        action6,
        ActionType::FunctionEval(FunctionEvalAction::MapFilter(MapFilterEvalAction {
            source_key: Box::from("k4".as_bytes()),
            source_old_value: Some(Box::from("v4".as_bytes())),
            source_new_value: Some(Box::from("v4-2".as_bytes())),
        }))
    );

    let mut actions = transform
        .run(vec![Input {
            id: second_cursor_id,
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

    assert_eq!(actions.next(), None);

    let mut actions = transform
        .run(vec![Input {
            id: id3,
            input: InputType::FunctionEval(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    old_key: Some(Box::from("k2-map-1".as_bytes())),
                    new_key: Some(Box::from("k2-map".as_bytes())),
                    value: Some(Box::from("v2-2-map".as_bytes())),
                }),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    assert_eq!(actions.next(), None);

    let mut actions = transform
        .run(vec![Input {
            id: id4,
            input: InputType::FunctionEval(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    old_key: Some(Box::from("k3-map".as_bytes())),
                    new_key: None,
                    value: None,
                }),
            }),
        }])
        .unwrap()
        .into_actions()
        .unwrap()
        .into_iter();

    assert_eq!(actions.next(), None);

    let mut actions = transform
        .run(vec![Input {
            id: id6,
            input: InputType::FunctionEval(FunctionEvalInput {
                body: FunctionEvalInputBody::MapFilter(MapFilterEvalInput {
                    old_key: Some(Box::from("k4-map".as_bytes())),
                    new_key: Some(Box::from("k4-map".as_bytes())),
                    value: Some(Box::from("v4-map".as_bytes())),
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
                        key: EncodedKeyJsonData::new_str("k2-map-1".to_string()),
                        if_not_present: None,
                        value: None
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
                    KeyValueUpdateJsonData {
                        key: EncodedKeyJsonData::new_str("k4-map".to_string()),
                        if_not_present: None,
                        value: Some(EncodedValueJsonData::new_str("v4-map".to_string()))
                    }
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

    let finish = transform
        .run(vec![Input {
            id: commit_id,
            input: InputType::DiffbeltCall(DiffbeltCallInput {
                body: DiffbeltResponseBody::Ok(()),
            }),
        }])
        .unwrap();

    assert_eq!(finish, TransformRunResult::Finish);
}
*/
