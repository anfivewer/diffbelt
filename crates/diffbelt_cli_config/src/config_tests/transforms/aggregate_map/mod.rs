use std::borrow::Cow;
use std::ops::Deref;
use std::rc::Rc;
use std::str::from_utf8;

use diffbelt_protos::protos::transform::map_filter::{MapFilterMultiInput, MapFilterMultiOutput};
use diffbelt_protos::{deserialize, OwnedSerialized};
use diffbelt_util::errors::NoStdErrorWrap;
use diffbelt_util::option::lift_result_from_option;
use diffbelt_util_no_std::cast::checked_usize_to_i32;
use diffbelt_util_no_std::slice::get_slice_offset_in_other_slice;
use diffbelt_wasm_binding::ptr::bytes::BytesSlice;
use diffbelt_yaml::YamlNode;

use crate::config_tests::error::{AssertError, TestError};
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
        ])
    }

    fn create(
        data: &'a Self::InitialData,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<Self, TestError> {
        if wasm_modules.len() != 4 {
            return Err(TestError::Panic(format!(
                "wasm_module has wrong size: {}",
                wasm_modules.len()
            )));
        }

        let (source_wasm, human_readable_wasm, map_wasm) = unsafe {
            (
                wasm_modules.get_unchecked(0),
                wasm_modules.get_unchecked(1),
                wasm_modules.get_unchecked(2),
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
        );

        // TODO: make AggregateFunctions

        Ok(Self {
            source_human_readable,
            aggregate_human_readable,
            map_filter,
        })
    }

    type Input = OwnedSerialized<'static, MapFilterMultiInput<'static>>;

    fn input_from_test_vars<'b>(&self, vars: &Rc<YamlNode>) -> Result<Self::Input, TestError> {
        let serialized =
            yaml_test_vars_to_map_filter_input(&self.source_human_readable, vars.as_ref())?;

        Ok(serialized)
    }

    type Output = (
        WasmVecHolder<'a>,
        Vec<(BytesSlice<WasmPtrImpl>, Option<BytesSlice<WasmPtrImpl>>)>,
    );

    fn input_to_output(&'a self, input: Self::Input) -> Result<Self::Output, TestError> {
        let result_holder = self.map_filter.instance.alloc_vec_holder()?;

        let bytes_result = self.map_filter.call(input.as_bytes(), &result_holder)?;

        let update_record_slices = bytes_result.observe_bytes(|bytes| {
            let multi_output =
                deserialize::<MapFilterMultiOutput>(bytes).map_err(TestError::InvalidFlatbuffer)?;

            let Some(update_records) = multi_output.target_update_records() else {
                return Ok(Vec::new());
            };

            let mut update_record_slices = Vec::with_capacity(update_records.len());

            for update_record in update_records {
                let key = update_record
                    .key()
                    .ok_or_else(|| TestError::Unspecified("RecordUpdate: no key".to_string()))?;
                let key = key.bytes();

                let value = update_record.value();
                let value = value.map(|x| x.bytes());

                let key_offset =
                    get_slice_offset_in_other_slice(bytes, key).map_err(NoStdErrorWrap::from)?;

                let value_offset = value.map(|value| {
                    get_slice_offset_in_other_slice(bytes, value).map_err(NoStdErrorWrap::from)
                });
                let value_offset = lift_result_from_option(value_offset)?;

                let key_ptr = bytes_result.bytes_offset_to_ptr(key_offset)?;

                let value_ptr =
                    value_offset.map(|value_offset| bytes_result.bytes_offset_to_ptr(value_offset));
                let value_ptr = lift_result_from_option(value_ptr)?;

                let value_slice = value_ptr.map(|value_ptr| BytesSlice::<WasmPtrImpl> {
                    ptr: value_ptr.into(),
                    len: checked_usize_to_i32(
                        value
                            .expect("value should be present if value_ptr present")
                            .len(),
                    ),
                });

                update_record_slices.push((
                    BytesSlice::<WasmPtrImpl> {
                        ptr: key_ptr.into(),
                        len: checked_usize_to_i32(key.len()),
                    },
                    value_slice,
                ));
            }

            Ok::<_, TestError>(update_record_slices)
        })?;

        Ok((result_holder, update_record_slices))
    }

    type ActualOutput = Vec<(WasmVecHolder<'a>, Option<WasmVecHolder<'a>>)>;

    fn output_to_actual_output(
        &self,
        output: Self::Output,
    ) -> Result<Self::ActualOutput, TestError> {
        let (result_bytes_holder, update_record_slices) = output;

        let instance = self.target_human_readable.instance;

        let mut kv_holders = Vec::with_capacity(update_record_slices.len());

        for (key, value) in update_record_slices {
            let key_vec_holder = instance.alloc_vec_holder()?;

            () = self
                .target_human_readable
                .call_bytes_to_key(&key, &key_vec_holder)?;

            let value_vec_holder = value.map(|value| {
                let value_vec_holder = instance.alloc_vec_holder()?;

                () = self
                    .target_human_readable
                    .call_bytes_to_value(&value, &value_vec_holder)?;

                Ok::<_, TestError>(value_vec_holder)
            });
            let value_vec_holder = lift_result_from_option(value_vec_holder)?;

            kv_holders.push((key_vec_holder, value_vec_holder));
        }

        drop(result_bytes_holder);

        Ok(kv_holders)
    }

    type ExpectedOutput = Vec<(&'a str, Option<&'a str>)>;

    fn expected_output_from_test_vars(
        &self,
        vars: &'a Rc<YamlNode>,
    ) -> Result<Self::ExpectedOutput, TestError> {
        let expected_output = yaml_test_output_to_map_filter_expected_output(vars.deref())?;

        Ok(expected_output)
    }

    fn compare_actual_and_expected_output(
        &self,
        actual: &Self::ActualOutput,
        expected: &Self::ExpectedOutput,
    ) -> Result<Option<AssertError>, TestError> {
        let result = self
            .target_human_readable
            .instance
            .enter_memory_observe_context(|memory| {
                let mut expected_iter = expected.iter();

                for (key, value) in actual {
                    let key = memory.vec_view(key)?;
                    let value = value.as_ref().map(|x| memory.vec_view(x));
                    let value = lift_result_from_option(value)?;

                    let key = from_utf8(key.as_ref())?;
                    let key = key.trim();
                    let value = value.as_ref().map(|x| from_utf8(x.as_ref()));
                    let value = lift_result_from_option(value)?;
                    let value = value.map(|x| x.trim());

                    let Some((expected_key, expected_value)) = expected_iter.next() else {
                        return Ok(Some(AssertError::ValueMissmatch {
                            message: Cow::Borrowed("Extra actual key"),
                            actual: Some(key.to_string()),
                            expected: None,
                        }));
                    };

                    let expected_key = *expected_key;
                    let expected_value = *expected_value;

                    if key != expected_key {
                        return Ok(Some(AssertError::ValueMissmatch {
                            message: Cow::Borrowed("Key diff"),
                            actual: Some(key.to_string()),
                            expected: Some(expected_key.to_string()),
                        }));
                    }

                    if let (Some(actual_value), Some(expected_value)) = (value, expected_value) {
                        if actual_value == expected_value {
                            continue;
                        }

                        return Ok(Some(AssertError::ValueMissmatch {
                            message: Cow::Borrowed("Value diff"),
                            actual: Some(actual_value.to_string()),
                            expected: Some(expected_value.to_string()),
                        }));
                    }

                    if let (None, None) = (value, expected_value) {
                        continue;
                    }

                    return Ok(Some(AssertError::ValueMissmatch {
                        message: Cow::Borrowed("Value diff"),
                        actual: value.map(|x| x.to_string()),
                        expected: expected_value.map(|x| x.to_string()),
                    }));
                }

                Ok::<_, TestError>(None)
            })?;

        Ok(result)
    }
}
