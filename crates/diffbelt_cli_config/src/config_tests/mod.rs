pub mod run;
pub mod value;

use crate::config_tests::value::{construct_value_from_yaml, YamlValueConstructionError};
use crate::interpreter::error::InterpreterError;
use crate::interpreter::function::Function;
use crate::interpreter::value::ValueHolder;
use crate::interpreter::var::{Var, VarDef};
use crate::CliConfig;
use diffbelt_yaml::YamlNodeRc;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct TestSuite {
    pub tests: Vec<SingleTest>,
}

#[derive(Debug, Deserialize)]
pub struct SingleTest {
    pub name: Rc<str>,
    pub vars: HashMap<Rc<str>, YamlNodeRc>,
    #[serde(rename = "return")]
    pub value: YamlNodeRc,
}

pub type AssertError = Rc<str>;

#[derive(Debug)]
pub enum TestError {
    InvalidName,
    Unspecified(String),
    YamlValueConstruction(YamlValueConstructionError),
    Interpreter(InterpreterError),
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
    pub fn run_tests(&self) -> Vec<TestResult> {
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

            let (input_vars, code) = match first {
                "transforms" => {
                    let Some(transform_name) = split.next() else {
                        push_error!("Specify which transform you want to test".to_string());
                        continue 'outer;
                    };

                    let Some(transform) = self.transforms.iter().find(|transform| {
                        transform
                            .name
                            .as_ref()
                            .map(|name| name.deref() == transform_name)
                            .unwrap_or(false)
                    }) else {
                        push_error!(format!(
                            "Transform {transform_name} not found, add name to it's declaration"
                        ));
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
                            if let Some(map_filter) = transform.map_filter.as_ref() {
                                (
                                    [(Rc::from("source"), VarDef::anonymous_string())]
                                        .into_iter()
                                        .collect::<IndexMap<Rc<str>, VarDef>>(),
                                    map_filter,
                                )
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

            let function = match Function::from_code(self, code, Some(input_vars)) {
                Ok(x) => x,
                Err(err) => {
                    result.push(TestResult {
                        name: name.clone(),
                        result: Err(TestError::Interpreter(err)),
                    });
                    continue 'outer;
                }
            };

            // let input_vars = vec![(
            //     Rc::from("source"),
            //     Var {
            //         def: VarDef::anonymous_string(),
            //         value: Some(ValueHolder { value: Value::String(Rc::from("S 2023-02-20T21:42:48.822Z.000 worker258688:middlewares handleFull updateType:edited_message ms:27 |some extra|another extra")) }),
            //     },
            // )]
            //     .into_iter().collect();
            //
            // let actual_value = function.call(input_vars).expect("function execution");

            let mut single_tests = Vec::with_capacity(tests.len());

            'test: for test in tests {
                let SingleTest {
                    name,
                    vars,
                    value: expected_value,
                } = test;

                let mut input_vars = HashMap::with_capacity(vars.len());

                for (key, value) in vars {
                    let value = match construct_value_from_yaml(value.as_ref()) {
                        Ok(x) => x,
                        Err(err) => {
                            single_tests.push(SingleTestResult {
                                name: name.clone(),
                                result: Err(TestError::YamlValueConstruction(err)),
                            });
                            continue 'test;
                        }
                    };

                    input_vars.insert(
                        key.clone(),
                        Var {
                            def: VarDef::unknown(),
                            value: Some(ValueHolder { value }),
                        },
                    );
                }

                let expected_value = match construct_value_from_yaml(expected_value.as_ref()) {
                    Ok(x) => x,
                    Err(err) => {
                        single_tests.push(SingleTestResult {
                            name: name.clone(),
                            result: Err(TestError::YamlValueConstruction(err)),
                        });
                        continue 'test;
                    }
                };

                let actual_value = match function.call(input_vars) {
                    Ok(x) => x,
                    Err(err) => {
                        single_tests.push(SingleTestResult {
                            name: name.clone(),
                            result: Err(TestError::Interpreter(err)),
                        });
                        continue;
                    }
                };

                if actual_value == expected_value {
                    single_tests.push(SingleTestResult {
                        name: name.clone(),
                        result: Ok(Some(Rc::from("value missmatch"))),
                    });
                } else {
                    single_tests.push(SingleTestResult {
                        name: name.clone(),
                        result: Ok(None),
                    });
                }
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
    use diffbelt_yaml::parse_yaml;

    #[test]
    fn run_example_config_tests() {
        let config_str = include_str!("../../../../examples/cli-config.yaml");

        let docs = parse_yaml(config_str).expect("parsing");
        let doc = &docs[0];
        let config = CliConfig::from_yaml(doc).expect("reading");

        let results = config.run_tests();

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
