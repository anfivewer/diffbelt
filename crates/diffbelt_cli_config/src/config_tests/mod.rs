use std::borrow::Cow;
use std::ops::Deref;
use std::rc::Rc;

use serde::Deserialize;

use diffbelt_yaml::{YamlNode, YamlNodeRc};
use error::{AssertError, TestError};

use crate::{CliConfig, Collection};
use crate::config_tests::transforms::map_filter::MapFilterTransformTest;
use crate::wasm::{
    NewWasmInstanceOptions, WasmModuleInstance,
};

pub mod error;
pub mod run;
#[cfg(test)]
mod tests;
pub mod transforms;
pub mod value;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct TestSuite {
    pub tests: Vec<SingleTest>,
}

#[derive(Debug, Deserialize)]
pub struct SingleTest {
    pub name: Rc<str>,
    pub input: YamlNodeRc,
    pub output: YamlNodeRc,
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

pub struct TransformTestPreCreateOptions<'a, T> {
    pub source_collection: &'a Collection,
    pub target_collection: &'a Collection,
    pub data: T,
}

pub trait TransformTest<'a>: Sized {
    type ConstructorData;
    type InitialData;

    fn pre_create(
        options: TransformTestPreCreateOptions<'a, Self::ConstructorData>,
    ) -> Result<Self::InitialData, TestError>;

    fn required_wasm_modules(data: &'a Self::InitialData) -> Result<Vec<Cow<'a, str>>, TestError>;

    fn create(
        data: &'a Self::InitialData,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<Self, TestError>;

    type Input;
    fn input_from_test_vars(
        &self,
        vars: &Rc<YamlNode>,
    ) -> Result<Self::Input, TestError>;

    type Output;
    fn input_to_output(
        &'a self,
        input: Self::Input,
    ) -> Result<Self::Output, TestError>;

    type ActualOutput;
    fn output_to_actual_output(
        &self,
        output: Self::Output,
    ) -> Result<Self::ActualOutput, TestError>;

    type ExpectedOutput;
    fn expected_output_from_test_vars(
        &self,
        vars: &'a Rc<YamlNode>,
    ) -> Result<Self::ExpectedOutput, TestError>;

    fn compare_actual_and_expected_output(
        &self,
        actual: &Self::ActualOutput,
        expected: &Self::ExpectedOutput,
    ) -> Result<Option<AssertError>, TestError>;
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

            macro_rules! match_ok {
                ($expr:expr) => {
                    match $expr {
                        Ok(x) => x,
                        Err(err) => {
                            result.push(TestResult {
                                name: name.clone(),
                                result: Err(err.into()),
                            });
                            continue 'outer;
                        }
                    }
                };
            }

            let initial_data = match first {
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

                    let Some(source_collection) = self.collection_by_name(transform.source.deref())
                    else {
                        push_error!(format!("Source collection {} not found", transform.source));
                        continue 'outer;
                    };

                    let Some(target_collection) = self.collection_by_name(transform.target.deref())
                    else {
                        push_error!(format!(
                            "Target collection {} not found",
                            transform.target.deref()
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
                            if let Some(map_filter) = transform.map_filter_wasm.as_ref() {
                                match_ok!(<MapFilterTransformTest as TransformTest>::pre_create(
                                    TransformTestPreCreateOptions {
                                        source_collection,
                                        target_collection,
                                        data: map_filter,
                                    }
                                ))
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

            let required_wasm_modules = match_ok!(
                <MapFilterTransformTest as TransformTest>::required_wasm_modules(&initial_data)
            );

            let mut wasm_modules =
                Vec::<(&str, Rc<WasmModuleInstance>)>::with_capacity(required_wasm_modules.len());

            for name in &required_wasm_modules {
                let name = name.as_ref();

                if let Some(existing_module) = wasm_modules
                    .iter()
                    .find(|(module_name, _)| *module_name == name)
                    .map(|(_, wasm)| wasm.clone())
                {
                    wasm_modules.push((name, existing_module));
                    continue;
                }

                let wasm = match_ok!(self
                    .wasm
                    .get(name)
                    .ok_or_else(|| TestError::Unspecified(format!("no wasm module {name}"))));

                let instance = match_ok!(
                    wasm.new_wasm_instance(NewWasmInstanceOptions {
                        config_path: self.self_path.deref(),
                    })
                    .await
                );

                let instance = Rc::new(instance);

                wasm_modules.push((name, instance));
            }

            let wasm_modules: Vec<&WasmModuleInstance> =
                wasm_modules.iter().map(|(_, wasm)| wasm.deref()).collect();

            let mut transform_test = match_ok!(<MapFilterTransformTest as TransformTest>::create(
                &initial_data,
                wasm_modules
            ));

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
                    input: vars,
                    output: expected_value,
                } = test;

                let input = match_ok!(transform_test.input_from_test_vars(vars.deref()));
                let output = match_ok!(transform_test.input_to_output(input));

                let actual_output = match_ok!(transform_test.output_to_actual_output(output));
                let expected_output = match_ok!(
                    transform_test.expected_output_from_test_vars(expected_value.deref())
                );

                let comparison = match_ok!(transform_test
                    .compare_actual_and_expected_output(&actual_output, &expected_output));

                single_tests.push(SingleTestResult {
                    name: name.clone(),
                    result: Ok(comparison),
                });
            }

            result.push(TestResult {
                name: name.clone(),
                result: Ok(single_tests),
            });
        }

        result
    }
}
