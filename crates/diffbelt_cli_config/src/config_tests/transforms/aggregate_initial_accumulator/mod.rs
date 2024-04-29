use std::borrow::Cow;
use std::rc::Rc;
use std::str::from_utf8;

use crate::config_tests::compare::compare_strings;
use diffbelt_protos::protos::transform::aggregate::AggregateTargetInfo;
use diffbelt_protos::OwnedSerialized;
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_initial_accumulator::yaml_input::yaml_test_vars_to_aggregate_initial_accumulator_input;
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
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::WasmModuleInstance;

mod yaml_input;

pub struct AggregateInitialAccumulatorTransformTestCreator<'a> {
    data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
}

impl<'a> AggregateInitialAccumulatorTransformTestCreator<'a> {
    pub fn new(
        data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
    ) -> Result<TransformTestCreatorImpl<'a>, TestError> {
        Ok(TransformTestCreatorImpl::AggregateInitialAccumulator(
            AggregateInitialAccumulatorTransformTestCreator { data },
        ))
    }
}

impl<'a> TransformTestCreator<'a> for AggregateInitialAccumulatorTransformTestCreator<'a> {
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

        let target_human_readable = target_human_readable.expect("was required");

        Ok(TransformTestImpl::AggregateInitialAccumulator(
            AggregateInitialAccumulatorTransformTest {
                target_human_readable,
                aggregate_human_readable,
                aggregate,
            },
        ))
    }
}

pub struct AggregateInitialAccumulatorTransformTest<'a> {
    target_human_readable: HumanReadableFunctions<'a>,
    aggregate_human_readable: AggregateHumanReadableFunctions<'a>,
    aggregate: AggregateFunctions<'a>,
}

type Input = OwnedSerialized<'static, AggregateTargetInfo<'static>>;
type Output<'a> = WasmVecHolder<'a>;
type ActualOutput = String;
type ExpectedOutput<'a> = &'a str;

impl<'a> AggregateInitialAccumulatorTransformTest<'a> {
    async fn input_from_test_vars<'b>(&self, vars: &Rc<YamlNode>) -> Result<Input, TestError> {
        let serialized = yaml_test_vars_to_aggregate_initial_accumulator_input(
            &self.target_human_readable,
            vars.as_ref(),
        )
        .await?;

        Ok(serialized)
    }

    async fn input_to_output(&self, input: Input) -> Result<Output<'a>, TestError> {
        let holder = self.aggregate.instance.alloc_vec_holder().await?;

        () = self
            .aggregate
            .call_initial_accumulator(FlatbufferAnnotated::from(input.as_bytes()), &holder)
            .await?;

        Ok(holder)
    }

    async fn output_to_actual_output(&self, output: Output<'a>) -> Result<ActualOutput, TestError> {
        let slice = output.read_slice()?;
        let output_slice = self.aggregate.instance.alloc_vec_holder().await?;

        let result = self
            .aggregate_human_readable
            .call_bytes_to_accumulator(slice, &output_slice)
            .await?
            .observe_bytes(self.aggregate_human_readable.instance, |result| {
                Ok::<_, TestError>(String::from(from_utf8(result)?))
            })?;

        Ok(result)
    }

    fn expected_output_from_test_vars(
        &self,
        vars: &'a Rc<YamlNode>,
    ) -> Result<ExpectedOutput, TestError> {
        let output = vars.as_str().ok_or_else(|| {
            TestError::Unspecified("initial_accumulator output should be a string".to_string())
        })?;

        Ok(output)
    }

    fn compare_actual_and_expected_output(
        &self,
        actual: &ActualOutput,
        expected: &ExpectedOutput<'a>,
    ) -> Result<Option<AssertError>, TestError> {
        Ok(compare_strings(expected, actual))
    }
}

impl<'a> TransformTest<'a> for AggregateInitialAccumulatorTransformTest<'a> {
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
