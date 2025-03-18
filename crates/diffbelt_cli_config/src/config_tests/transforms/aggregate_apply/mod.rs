use std::borrow::Cow;
use std::ops::Deref;
use std::rc::Rc;
use std::str::from_utf8;

use diffbelt_protos::protos::impls::AggregateApplyOutputProto;
use diffbelt_protos::OwnedSerialized;
use diffbelt_wasm_binding::error_code::ErrorCode;
use diffbelt_yaml::YamlNode;

use crate::config_tests::compare::compare_strings;
use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_apply::yaml_input::yaml_test_vars_to_aggregate_apply_input;
use crate::config_tests::transforms::aggregate_util::{
    create_test_aggregate_functions, require_wasm_modules_aggregate,
};
use crate::config_tests::transforms::{
    TransformTest, TransformTestCreator, TransformTestCreatorImpl, TransformTestImpl,
    TransformTestPreCreateOptions,
};
use crate::transforms::aggregate::Aggregate;
use crate::wasm::aggregate::AggregateFunctions;
use crate::wasm::human_readable::aggregate::AggregateHumanReadableFunctions;
use crate::wasm::human_readable::HumanReadableFunctions;
use crate::wasm::WasmModuleInstance;
use crate::wasm::error::WasmError;

mod yaml_input;

pub struct AggregateApplyTransformTestCreator<'a> {
    data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
}

impl<'a> AggregateApplyTransformTestCreator<'a> {
    pub fn new(
        data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
    ) -> Result<TransformTestCreatorImpl<'a>, TestError> {
        Ok(TransformTestCreatorImpl::AggregateApply(
            AggregateApplyTransformTestCreator { data },
        ))
    }
}

impl<'a> TransformTestCreator<'a> for AggregateApplyTransformTestCreator<'a> {
    fn required_wasm_modules(&self) -> Result<Vec<Cow<'a, str>>, TestError> {
        let TransformTestPreCreateOptions {
            source_collection: _,
            target_collection,
            data,
        } = self.data;

        assert_eq!(
            Some(data.wasm.as_str()),
            data.human_readable.as_ref().map(|x| x.wasm.as_str()),
            "not implemented yet, need copy accumulator bytes"
        );

        require_wasm_modules_aggregate(None, Some(target_collection), data)
    }

    async fn create(
        self,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<TransformTestImpl<'a>, TestError> {
        let TransformTestPreCreateOptions {
            source_collection: _,
            target_collection,
            data,
        } = self.data;

        let (_, target_human_readable, aggregate, aggregate_human_readable) =
            create_test_aggregate_functions(None, Some(target_collection), data, wasm_modules)
                .await?;

        let target_human_readable = target_human_readable.expect("was requested");

        Ok(TransformTestImpl::AggregateApply(
            AggregateApplyTransformTest {
                target_human_readable,
                aggregate_human_readable,
                aggregate,
            },
        ))
    }
}

pub struct AggregateApplyTransformTest<'a> {
    target_human_readable: HumanReadableFunctions<'a>,
    aggregate_human_readable: AggregateHumanReadableFunctions<'a>,
    aggregate: AggregateFunctions<'a>,
}

#[derive(Debug, Eq, PartialEq)]
enum ExpectedError {
    WasmErrorCode(ErrorCode),
}

type Input = Vec<u8>;
type Output<'a> = Result<OwnedSerialized<AggregateApplyOutputProto>, ExpectedError>;
type ActualOutput = Result<Option<String>, ExpectedError>;
type ExpectedOutput<'a> = Result<Option<&'a str>, ExpectedError>;

impl<'a> AggregateApplyTransformTest<'a> {
    async fn input_from_test_vars<'b>(&self, vars: &Rc<YamlNode>) -> Result<Input, TestError> {
        let accumulator_and_input =
            yaml_test_vars_to_aggregate_apply_input(&self.aggregate_human_readable, vars.as_ref())
                .await?;

        Ok(accumulator_and_input)
    }

    async fn input_to_output(&self, input: Input) -> Result<Output<'a>, TestError> {
        let accumulator = input;

        let holder = self.aggregate.instance.alloc_vec_holder().await?;
        let () = holder.replace_with_slice(accumulator.as_slice()).await?;

        let output = self.aggregate.call_apply(&holder, &mut None).await;

        match output {
            Ok(x) => Ok(Ok(x)),
            Err(err) => {
                let WasmError::AggregateApplyErrorCode(code) = err else {
                    return Err(err.into());
                };

                Ok(Err(ExpectedError::WasmErrorCode(code)))
            }
        }
    }

    async fn output_to_actual_output(&self, output: Output<'a>) -> Result<ActualOutput, TestError> {
        let output = match output {
            Ok(x) => x,
            Err(err) => {
                return Ok(Err(err));
            }
        };

        let target_value = output.data().target_value();

        let result = match target_value {
            None => None,
            Some(target_value) => {
                let holder = self.aggregate.instance.alloc_vec_holder().await?;
                let () = holder.replace_with_slice(target_value.bytes()).await?;

                let input_slice = holder.read_slice()?;
                let output_holder = self.aggregate.instance.alloc_vec_holder().await?;

                let result = self
                    .target_human_readable
                    .call_bytes_to_value(input_slice, &output_holder)
                    .await?
                    .observe_bytes(self.target_human_readable.instance, |result| {
                        Ok::<_, TestError>(String::from(from_utf8(result)?))
                    })?;

                Some(result)
            }
        };

        Ok(Ok(result))
    }

    fn expected_output_from_test_vars(
        &self,
        vars: &'a Rc<YamlNode>,
    ) -> Result<ExpectedOutput, TestError> {
        if let Some(tag) = vars.tag.as_ref() {
            let tag = tag.deref();
            if tag == "!none" {
                return Ok(Ok(None));
            }

            if tag == "!should_safe_fail" {
                return Ok(Err(ExpectedError::WasmErrorCode(ErrorCode::SafeFail)));
            }
        }

        let output = vars.as_str().ok_or_else(|| {
            TestError::Unspecified("reduce output should be a string".to_string())
        })?;

        Ok(Ok(Some(output)))
    }

    fn compare_actual_and_expected_output(
        &self,
        actual: &ActualOutput,
        expected: &ExpectedOutput<'a>,
    ) -> Result<Option<AssertError>, TestError> {
        let (expected, actual) = match expected {
            Ok(expected) => {
                let actual = match actual {
                    Ok(x) => x,
                    Err(err) => {
                        return Ok(Some(AssertError::ValueMissmatch {
                            message: Cow::Borrowed("Got error"),
                            expected: None,
                            actual: Some(format!("{err:?}")),
                        }));
                    }
                };

                (expected, actual)
            }
            Err(expected_err) => {
                let actual_err = match actual {
                    Ok(actual) => {
                        return Ok(Some(AssertError::ExpectedErrorButSucceed {
                            actual: actual.clone(),
                        }));
                    }
                    Err(err) => err,
                };

                if expected_err == actual_err {
                    return Ok(None);
                }

                return Ok(Some(AssertError::ValueMissmatch {
                    message: Cow::Borrowed("Error mismatch"),
                    expected: Some(format!("{expected_err:?}")),
                    actual: Some(format!("{actual_err:?}")),
                }));
            }
        };

        if actual.is_none() == expected.is_none() {
            return Ok(None);
        }

        let expected = expected.unwrap_or("");
        let actual = actual.as_ref().map(|x| x.as_str()).unwrap_or("");

        Ok(compare_strings(expected, actual))
    }
}

impl<'a> TransformTest<'a> for AggregateApplyTransformTest<'a> {
    async fn test(
        &self,
        input: &Rc<YamlNode>,
        expected_output: &Rc<YamlNode>,
    ) -> Result<Option<AssertError>, TestError> {
        let input = self.input_from_test_vars(&input).await?;
        let output = self.input_to_output(input).await?;
        let actual_output = self.output_to_actual_output(output).await?;
        let expected_output = self.expected_output_from_test_vars(&expected_output)?;
        let comparison =
            self.compare_actual_and_expected_output(&actual_output, &expected_output)?;

        Ok(comparison)
    }
}
