pub mod run;
pub mod value;

use crate::config_tests::value::{construct_value_from_yaml, YamlValueConstructionError};
use crate::interpreter::error::InterpreterError;
use crate::interpreter::function::Function;
use crate::interpreter::value::{Value, ValueHolder};
use crate::interpreter::var::{Var, VarDef};
use crate::{CliConfig, CollectionValueFormat};
use diffbelt_yaml::YamlNodeRc;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Deref;

use crate::formats::yaml_map_filter::YamlTestVarsError;
use crate::transforms::map_filter::{MapFilterWasm, MapFilterYaml};
use crate::wasm::{MapFilterFunction, NewWasmInstanceOptions, WasmError, WasmModuleInstance};
use diffbelt_protos::OwnedSerialized;
use std::rc::Rc;
use either::Either;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct TestSuite {
    pub tests: Vec<SingleTest>,
}

#[derive(Debug, Deserialize)]
pub struct SingleTest {
    pub name: Rc<str>,
    pub vars: YamlNodeRc,
    #[serde(rename = "return")]
    pub value: YamlNodeRc,
}

#[derive(Debug)]
pub enum AssertError {
    ValueMissmatch { expected: Value, actual: Value },
}

#[derive(Error, Debug)]
pub enum TestError {
    #[error("InvalidName")]
    InvalidName,
    #[error("{0}")]
    Unspecified(String),
    #[error("{0:?}")]
    YamlValueConstruction(YamlValueConstructionError),
    #[error("{0:?}")]
    Interpreter(InterpreterError),
    #[error(transparent)]
    Wasm(#[from] WasmError),
    #[error(transparent)]
    YamlTestVars(#[from] YamlTestVarsError),
}

impl From<Either<TestError, WasmError>> for TestError {
    fn from(value: Either<TestError, WasmError>) -> Self {
        match value {
            Either::Left(err) => err,
            Either::Right(err) => err.into(),
        }
    }
}

#[derive(Debug)]
pub struct SingleTestResult {
    pub name: Rc<str>,
    pub result: Result<Option<AssertError>, TestError>,
}

#[derive(Debug)]
pub struct TestResult {
    pub name: Rc<str>,
    pub result: Result<Vec<SingleTestResult>, TestError>,
}

impl CliConfig {
    pub async fn run_tests(&self) -> Vec<TestResult> {
        let mut result = Vec::new();

        'outer: for (name, suite) in self.tests.iter() {
            let TestSuite { tests } = suite;

            let mut split = name.split('.');
            let first = split.next().expect("first split cannot be empty");

            macro_rules! push_error {
                ( $err:expr ) => {
                    result.push(TestResult {
                        name: name.clone(),
                        result: Err(TestError::Unspecified($err)),
                    });
                };
            }

            enum TransformType<'a> {
                MapFilter {
                    source_format: CollectionValueFormat,
                    map_filter: &'a MapFilterWasm,
                },
            }

            let code_def = match first {
                "transforms" => {
                    let Some(transform_name) = split.next() else {
                        push_error!("Specify which transform you want to test".to_string());
                        continue 'outer;
                    };

                    let Some(transform) = self.transform_by_name(transform_name) else {
                        push_error!(format!(
                            "Transform {transform_name} not found, add name to it's declaration"
                        ));
                        continue 'outer;
                    };

                    let Some(from_collection) = self.collection_by_name(transform.from.deref())
                    else {
                        push_error!(format!("Collection {} not found", transform.from));
                        continue 'outer;
                    };

                    let Some(transform_method) = split.next() else {
                        push_error!("Specify transform method which contains code".to_string());
                        continue 'outer;
                    };

                    let None = split.next() else {
                        push_error!(
                            "After transform method there should not be anything".to_string()
                        );
                        continue 'outer;
                    };

                    match transform_method {
                        "map_filter" => {
                            if let Some(map_filter) = transform.map_filter_wasm.as_ref() {
                                TransformType::MapFilter {
                                    source_format: from_collection.format,
                                    map_filter,
                                }
                            } else {
                                push_error!(format!("Transform {transform_name} does not contain {transform_method}"));
                                continue 'outer;
                            }
                        }
                        _ => {
                            push_error!(format!("Unknown transform method {transform_method}"));
                            continue 'outer;
                        }
                    }
                }
                _ => {
                    push_error!("Functions tests are not supported yet".to_string());
                    continue 'outer;
                }
            };

            enum TransformTypeRuntime<'a> {
                MapFilter {
                    source_format: CollectionValueFormat,
                    map_filter: &'a MapFilterWasm,
                    instance: WasmModuleInstance,
                },
            }

            enum TransformTypeFunction<'a> {
                MapFilter { fun: MapFilterFunction<'a> },
            }

            let mut runtime = match code_def {
                TransformType::MapFilter {
                    source_format,
                    map_filter,
                } => {
                    let MapFilterWasm {
                        mark,
                        module_name,
                        method_name,
                    } = map_filter;

                    let Some(wasm_mod_def) = self.wasm.get(module_name.as_str()) else {
                        push_error!(format!("No wasm module {module_name} defined"));
                        continue 'outer;
                    };

                    let instance = match wasm_mod_def
                        .new_wasm_instance(NewWasmInstanceOptions {
                            config_path: self.self_path.deref(),
                        })
                        .await
                    {
                        Ok(x) => x,
                        Err(err) => {
                            result.push(TestResult {
                                name: name.clone(),
                                result: Err(TestError::Wasm(err)),
                            });
                            continue 'outer;
                        }
                    };

                    TransformTypeRuntime::MapFilter {
                        source_format,
                        map_filter,
                        instance,
                    }
                }
            };

            let (source_format, mut fun) = match &mut runtime {
                TransformTypeRuntime::MapFilter {
                    source_format,
                    map_filter,
                    instance,
                } => {
                    let fun = match instance.map_filter_function(map_filter.method_name.as_str()) {
                        Ok(x) => x,
                        Err(err) => {
                            result.push(TestResult {
                                name: name.clone(),
                                result: Err(TestError::Wasm(err)),
                            });
                            continue 'outer;
                        }
                    };

                    (*source_format, TransformTypeFunction::MapFilter { fun })
                }
            };

            let mut single_tests = Vec::with_capacity(tests.len());

            'test: for test in tests {
                macro_rules! match_ok {
                    ( $expr:expr ) => {
                        match $expr {
                            Ok(x) => x,
                            Err(err) => {
                                single_tests.push(SingleTestResult {
                                    name: name.clone(),
                                    result: Err(err.into()),
                                });
                                continue 'test;
                            }
                        }
                    };
                }

                let SingleTest {
                    name,
                    vars,
                    value: expected_value,
                } = test;

                let input = match_ok!(source_format.yaml_test_vars_to_map_filter_input(vars.as_ref()));

                match &mut fun {
                    TransformTypeFunction::MapFilter { fun } => {
                        let result = match_ok!(fun.call(input.data()));
                        let result = match_ok!(result.observe_bytes(|bytes| {
                            println!("result {bytes:?}");

                            Ok::<(), TestError>(())
                        }));
                    }
                }

                // let expected_value = match construct_value_from_yaml(expected_value.as_ref()) {
                //     Ok(x) => x,
                //     Err(err) => {
                //         single_tests.push(SingleTestResult {
                //             name: name.clone(),
                //             result: Err(TestError::YamlValueConstruction(err)),
                //         });
                //         continue 'test;
                //     }
                // };
                //
                // let actual_value = match function.call(input_vars) {
                //     Ok(x) => x,
                //     Err(err) => {
                //         single_tests.push(SingleTestResult {
                //             name: name.clone(),
                //             result: Err(TestError::Interpreter(err)),
                //         });
                //         continue;
                //     }
                // };
                //
                // if actual_value != expected_value {
                //     single_tests.push(SingleTestResult {
                //         name: name.clone(),
                //         result: Ok(Some(AssertError::ValueMissmatch {
                //             expected: expected_value,
                //             actual: actual_value,
                //         })),
                //     });
                // } else {
                //     single_tests.push(SingleTestResult {
                //         name: name.clone(),
                //         result: Ok(None),
                //     });
                // }

                single_tests.push(SingleTestResult {
                    name: name.clone(),
                    result: Ok(None),
                });

                break;
            }

            result.push(TestResult {
                name: name.clone(),
                result: Ok(single_tests),
            });
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::config_tests::{SingleTestResult, TestResult};
    use crate::CliConfig;
    use diffbelt_util::tokio_runtime::create_main_tokio_runtime;
    use diffbelt_yaml::parse_yaml;
    use std::rc::Rc;

    #[test]
    fn run_example_config_tests() {
        let runtime = create_main_tokio_runtime().unwrap();
        runtime.block_on(run_example_config_tests_inner());
    }

    async fn run_example_config_tests_inner() {
        let config_str = include_str!("../../../../examples/cli-config.yaml");

        let docs = parse_yaml(config_str).expect("parsing");
        let doc = &docs[0];
        let config = CliConfig::from_yaml(Rc::from("../../examples"), doc).expect("reading");

        let results = config.run_tests().await;

        let mut is_ok = true;

        for result in results {
            let TestResult {
                name: function_name,
                result,
            } = result;

            let results = match result {
                Ok(x) => x,
                Err(err) => {
                    println!("[FAIL] {function_name}: {:?}", err);
                    is_ok = false;
                    continue;
                }
            };

            for result in results {
                let SingleTestResult { name, result } = result;

                let result = match result {
                    Ok(x) => x,
                    Err(err) => {
                        println!("[FAIL] {function_name} > {name}: {:?}", err);
                        is_ok = false;
                        continue;
                    }
                };

                if let Some(err) = result {
                    println!("[FAIL] {function_name} > {name}: {:?}", err);
                    is_ok = false;
                } else {
                    println!("[ OK ] {function_name} > {name}");
                }
            }
        }

        assert!(is_ok);
    }
}
