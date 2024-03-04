mod yaml_input;

use std::borrow::Cow;
use std::ops::Deref;
use std::rc::Rc;
use std::str::from_utf8;

use diffbelt_protos::protos::transform::aggregate::AggregateMapMultiInput;
use diffbelt_protos::protos::transform::map_filter::{MapFilterMultiInput, MapFilterMultiOutput};
use diffbelt_protos::{deserialize, OwnedSerialized};
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util::option::lift_result_from_option;
use diffbelt_util_no_std::cast::checked_usize_to_i32;
use diffbelt_util_no_std::slice::get_slice_offset_in_other_slice;
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_map::yaml_input::yaml_test_vars_to_aggregate_map_input;
use crate::config_tests::{TransformTest, TransformTestPreCreateOptions};
use crate::transforms::aggregate::Aggregate;
use crate::transforms::wasm::WasmMethodDef;
use crate::wasm::aggregate::AggregateFunctions;
use crate::wasm::human_readable::aggregate::AggregateHumanReadableFunctions;
use crate::wasm::human_readable::HumanReadableFunctions;
use crate::wasm::memory::vector::WasmVecHolder;
use crate::wasm::types::WasmPtrImpl;
use crate::wasm::{MapFilterFunction, WasmModuleInstance};

pub struct AggregateMapTransformTest<'a> {
    source_human_readable: HumanReadableFunctions<'a>,
    aggregate_human_readable: AggregateHumanReadableFunctions<'a>,
    aggregate: AggregateFunctions<'a>,
}

impl<'a> TransformTest<'a> for AggregateMapTransformTest<'a> {
    type ConstructorData = &'a Aggregate;
    type InitialData = TransformTestPreCreateOptions<'a, Self::ConstructorData>;

    fn pre_create(
        options: TransformTestPreCreateOptions<'a, Self::ConstructorData>,
    ) -> Result<Self::InitialData, TestError> {
        Ok(options)
    }

    fn required_wasm_modules(data: &Self::InitialData) -> Result<Vec<Cow<str>>, TestError> {
        let TransformTestPreCreateOptions {
            source_collection,
            target_collection: _,
            data,
        } = data;

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

    fn create(
        data: &'a Self::InitialData,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<Self, TestError> {
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
        } = data;

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
        )?;

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
        )?;

        let aggregate = AggregateFunctions::new(
            map_wasm,
            data.map.method_name.as_str(),
            data.initial_accumulator.method_name.as_str(),
            data.reduce.method_name.as_str(),
            data.merge_accumulators.method_name.as_str(),
            data.apply.method_name.as_str(),
        )?;

        Ok(Self {
            source_human_readable,
            aggregate_human_readable,
            aggregate,
        })
    }

    type Input = OwnedSerialized<'static, AggregateMapMultiInput<'static>>;

    fn input_from_test_vars<'b>(&self, vars: &Rc<YamlNode>) -> Result<Self::Input, TestError> {
        let serialized =
            yaml_test_vars_to_aggregate_map_input(&self.source_human_readable, vars.as_ref())?;

        Ok(serialized)
    }

    type Output = (
        WasmVecHolder<'a>,
        Vec<(BytesSlice<WasmPtrImpl>, Option<BytesSlice<WasmPtrImpl>>)>,
    );

    fn input_to_output(&'a self, input: Self::Input) -> Result<Self::Output, TestError> {
        todo!()
    }

    type ActualOutput = Vec<(WasmVecHolder<'a>, Option<WasmVecHolder<'a>>)>;

    fn output_to_actual_output(
        &self,
        output: Self::Output,
    ) -> Result<Self::ActualOutput, TestError> {
        todo!()
    }

    type ExpectedOutput = Vec<(&'a str, Option<&'a str>)>;

    fn expected_output_from_test_vars(
        &self,
        vars: &'a Rc<YamlNode>,
    ) -> Result<Self::ExpectedOutput, TestError> {
        todo!()
    }

    fn compare_actual_and_expected_output(
        &self,
        actual: &Self::ActualOutput,
        expected: &Self::ExpectedOutput,
    ) -> Result<Option<AssertError>, TestError> {
        todo!()
    }
}
