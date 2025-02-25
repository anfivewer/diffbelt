use std::borrow::Cow;
use std::rc::Rc;
use std::str::from_utf8;

use diffbelt_protos::protos::impls::AggregateReduceInputProto;
use diffbelt_protos::protos::transform::aggregate::AggregateReduceInput;
use diffbelt_protos::OwnedSerialized;
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;
use diffbelt_yaml::YamlNode;

use crate::config_tests::compare::compare_strings;
use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_reduce::yaml_input::yaml_test_vars_to_aggregate_reduce_input;
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
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::WasmModuleInstance;

mod yaml_input;

pub struct AggregateReduceTransformTestCreator<'a> {
    data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
}

impl<'a> AggregateReduceTransformTestCreator<'a> {
    pub fn new(
        data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
    ) -> Result<TransformTestCreatorImpl<'a>, TestError> {
        Ok(TransformTestCreatorImpl::AggregateReduce(
            AggregateReduceTransformTestCreator { data },
        ))
    }
}

impl<'a> TransformTestCreator<'a> for AggregateReduceTransformTestCreator<'a> {
    fn required_wasm_modules(&self) -> Result<Vec<Cow<'a, str>>, TestError> {
        let TransformTestPreCreateOptions {
            source_collection: _,
            target_collection: _,
            data,
        } = self.data;

        assert_eq!(
            Some(data.wasm.as_str()),
            data.human_readable.as_ref().map(|x| x.wasm.as_str()),
            "not implemented yet, need copy accumulator bytes"
        );

        require_wasm_modules_aggregate(None, None, data)
    }

    async fn create(
        self,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<TransformTestImpl<'a>, TestError> {
        let TransformTestPreCreateOptions {
            source_collection: _,
            target_collection: _,
            data,
        } = self.data;

        let (_, _, aggregate, aggregate_human_readable) =
            create_test_aggregate_functions(None, None, data, wasm_modules).await?;

        Ok(TransformTestImpl::AggregateReduce(
            AggregateReduceTransformTest {
                aggregate_human_readable,
                aggregate,
            },
        ))
    }
}

pub struct AggregateReduceTransformTest<'a> {
    aggregate_human_readable: AggregateHumanReadableFunctions<'a>,
    aggregate: AggregateFunctions<'a>,
}

type Input = (Vec<u8>, OwnedSerialized<AggregateReduceInputProto>);
type Output<'a> = WasmVecHolder<'a>;
type ActualOutput = String;
type ExpectedOutput<'a> = &'a str;

impl<'a> AggregateReduceTransformTest<'a> {
    async fn input_from_test_vars<'b>(&self, vars: &Rc<YamlNode>) -> Result<Input, TestError> {
        let accumulator_and_input =
            yaml_test_vars_to_aggregate_reduce_input(&self.aggregate_human_readable, vars.as_ref())
                .await?;

        Ok(accumulator_and_input)
    }

    async fn input_to_output(&self, input: Input) -> Result<Output<'a>, TestError> {
        let (accumulator, input) = input;

        let holder = self.aggregate.instance.alloc_vec_holder().await?;
        () = holder.replace_with_slice(accumulator.as_slice()).await?;

        () = self
            .aggregate
            .call_reduce(FlatbufferAnnotated::from(input.as_bytes()), &holder)
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
            TestError::Unspecified("reduce output should be a string".to_string())
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

impl<'a> TransformTest<'a> for AggregateReduceTransformTest<'a> {
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
