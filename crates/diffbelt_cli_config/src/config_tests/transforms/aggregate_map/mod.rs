mod yaml_input;

use diffbelt_protos::protos::transform::aggregate::AggregateMapMultiInput;
use diffbelt_protos::OwnedSerialized;
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use diffbelt_yaml::YamlNode;
use std::borrow::Cow;
use std::ops::Deref;
use std::rc::Rc;

use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_map::yaml_input::yaml_test_vars_to_aggregate_map_input;
use crate::config_tests::transforms::{
    TransformTest, TransformTestCreator, TransformTestCreatorImpl, TransformTestImpl,
    TransformTestPreCreateOptions,
};
use crate::transforms::aggregate::Aggregate;
use crate::transforms::wasm::WasmMethodDef;
use crate::wasm::aggregate::AggregateFunctions;
use crate::wasm::human_readable::aggregate::AggregateHumanReadableFunctions;
use crate::wasm::human_readable::HumanReadableFunctions;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::WasmPtrImpl;
use crate::wasm::WasmModuleInstance;

pub struct AggregateMapTransformTestCreator<'a> {
    data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
}

impl<'a> AggregateMapTransformTestCreator<'a> {
    pub fn new(
        data: TransformTestPreCreateOptions<'a, &'a Aggregate>,
    ) -> Result<TransformTestCreatorImpl<'a>, TestError> {
        Ok(TransformTestCreatorImpl::AggregateMap(
            AggregateMapTransformTestCreator { data },
        ))
    }
}

impl<'a> TransformTestCreator<'a> for AggregateMapTransformTestCreator<'a> {
    fn required_wasm_modules(&self) -> Result<Vec<Cow<'a, str>>, TestError> {
        let TransformTestPreCreateOptions {
            source_collection,
            target_collection: _,
            data,
        } = self.data;

        let Some(source) = &source_collection.human_readable else {
            return Err(TestError::SourceHasNoHumanReadableFunctions);
        };
        let Some(aggregate) = &data.human_readable else {
            return Err(TestError::AggregateHasNoHumanReadableFunctions);
        };

        Ok(vec![
            Cow::Borrowed(source.wasm.as_str()),
            Cow::Borrowed(aggregate.wasm.as_str()),
            Cow::Borrowed(data.map.module_name.as_str()),
            Cow::Borrowed(data.initial_accumulator.module_name.as_str()),
            Cow::Borrowed(data.reduce.module_name.as_str()),
            Cow::Borrowed(data.merge_accumulators.module_name.as_str()),
            Cow::Borrowed(data.apply.module_name.as_str()),
        ])
    }

    async fn create(
        self,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<TransformTestImpl<'a>, TestError> {
        if wasm_modules.len() != 7 {
            return Err(TestError::Panic(format!(
                "wasm_module has wrong size: {}",
                wasm_modules.len()
            )));
        }

        let (
            source_wasm,
            human_readable_wasm,
            map_wasm,
            initial_accumulator_wasm,
            reduce_wasm,
            merge_accumulators_wasm,
            apply_wasm,
        ) = unsafe {
            (
                wasm_modules.get_unchecked(0),
                wasm_modules.get_unchecked(1),
                wasm_modules.get_unchecked(2),
                wasm_modules.get_unchecked(3),
                wasm_modules.get_unchecked(4),
                wasm_modules.get_unchecked(5),
                wasm_modules.get_unchecked(6),
            )
        };

        let TransformTestPreCreateOptions {
            source_collection,
            target_collection,
            data,
        } = self.data;

        let source_human_readable = source_collection
            .human_readable
            .as_ref()
            .expect("already checked");
        let aggregate_human_readable = data.human_readable.as_ref().expect("already checked");

        let source_human_readable = source_wasm.human_readable_functions(
            source_human_readable.key_to_bytes.as_str(),
            source_human_readable.bytes_to_key.as_str(),
            source_human_readable.value_to_bytes.as_str(),
            source_human_readable.bytes_to_value.as_str(),
        ).await?;

        let aggregate_human_readable = AggregateHumanReadableFunctions::new(
            human_readable_wasm,
            aggregate_human_readable
                .mapped_key_from_bytes
                .as_ref()
                .ok_or_else(|| {
                    TestError::Unspecified(
                        "No mapped_key_from_bytes human readable function".to_string(),
                    )
                })?
                .as_str(),
            aggregate_human_readable
                .mapped_value_from_bytes
                .as_ref()
                .ok_or_else(|| {
                    TestError::Unspecified(
                        "No mapped_value_from_bytes human readable function".to_string(),
                    )
                })?
                .as_str(),
        )
        .await?;

        let aggregate = AggregateFunctions::new(
            map_wasm,
            data.map.method_name.as_str(),
            data.initial_accumulator.method_name.as_str(),
            data.reduce.method_name.as_str(),
            data.merge_accumulators.method_name.as_str(),
            data.apply.method_name.as_str(),
        )
        .await?;

        Ok(TransformTestImpl::AggregateMap(AggregateMapTransformTest {
            source_human_readable,
            aggregate_human_readable,
            aggregate,
        }))
    }
}

pub struct AggregateMapTransformTest<'a> {
    source_human_readable: HumanReadableFunctions<'a>,
    aggregate_human_readable: AggregateHumanReadableFunctions<'a>,
    aggregate: AggregateFunctions<'a>,
}

type Input = OwnedSerialized<'static, AggregateMapMultiInput<'static>>;
type Output<'a> = (
    WasmVecHolder<'a>,
    Vec<(BytesSlice<WasmPtrImpl>, Option<BytesSlice<WasmPtrImpl>>)>,
);
type ActualOutput<'a> = Vec<(WasmVecHolder<'a>, Option<WasmVecHolder<'a>>)>;
type ExpectedOutput<'a> = Vec<(&'a str, Option<&'a str>)>;

impl<'a> AggregateMapTransformTest<'a> {
    async fn input_from_test_vars<'b>(&self, vars: &Rc<YamlNode>) -> Result<Input, TestError> {
        let serialized =
            yaml_test_vars_to_aggregate_map_input(&self.source_human_readable, vars.as_ref())
                .await?;

        Ok(serialized)
    }

    fn input_to_output(&'a self, input: Input) -> Result<Output<'a>, TestError> {
        todo!()
    }

    fn output_to_actual_output(&self, output: Output<'a>) -> Result<ActualOutput<'a>, TestError> {
        todo!()
    }

    fn expected_output_from_test_vars(
        &self,
        vars: &'a Rc<YamlNode>,
    ) -> Result<ExpectedOutput<'a>, TestError> {
        todo!()
    }

    fn compare_actual_and_expected_output(
        &self,
        actual: &ActualOutput<'a>,
        expected: &ExpectedOutput<'a>,
    ) -> Result<Option<AssertError>, TestError> {
        todo!()
    }
}

impl<'a> TransformTest<'a> for AggregateMapTransformTest<'a> {
    async fn test(
        &self,
        input: &Rc<YamlNode>,
        expected_output: &Rc<YamlNode>,
    ) -> Result<Option<AssertError>, TestError> {
        let input = self.input_from_test_vars(&input).await?;
        let output = self.input_to_output(input)?;
        let actual_output = self.output_to_actual_output(output)?;
        let expected_output = self.expected_output_from_test_vars(&expected_output)?;
        let comparison =
            self.compare_actual_and_expected_output(&actual_output, &expected_output)?;

        Ok(comparison)
    }
}
