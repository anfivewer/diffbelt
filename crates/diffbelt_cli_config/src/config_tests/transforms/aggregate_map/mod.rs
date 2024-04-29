use std::borrow::Cow;
use std::rc::Rc;
use std::str::from_utf8;

use diffbelt_protos::protos::transform::aggregate::{
    AggregateMapMultiInput, AggregateMapMultiOutput,
};
use diffbelt_protos::OwnedSerialized;
use diffbelt_wasm_binding::annotations::FlatbufferAnnotated;
use diffbelt_yaml::{YamlMapping, YamlMark, YamlNode, YamlNodeValue, YamlScalar, YamlSequence};

use crate::call_human_readable_conversion;
use crate::config_tests::compare::compare_strings;
use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_map::yaml_input::yaml_test_vars_to_aggregate_map_input;
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

mod yaml_input;

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

        require_wasm_modules_aggregate(Some(source_collection), None, data)
    }

    async fn create(
        self,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<TransformTestImpl<'a>, TestError> {
        let TransformTestPreCreateOptions {
            source_collection,
            target_collection: _,
            data,
        } = self.data;

        let (source_human_readable, _, aggregate, aggregate_human_readable) =
            create_test_aggregate_functions(Some(source_collection), None, data, wasm_modules)
                .await?;

        let source_human_readable = source_human_readable.expect("was required");

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
type Output = OwnedSerialized<'static, AggregateMapMultiOutput<'static>>;
type ActualOutput = String;
type ExpectedOutput = String;

impl<'a> AggregateMapTransformTest<'a> {
    async fn input_from_test_vars<'b>(&self, vars: &Rc<YamlNode>) -> Result<Input, TestError> {
        let serialized =
            yaml_test_vars_to_aggregate_map_input(&self.source_human_readable, vars.as_ref())
                .await?;

        Ok(serialized)
    }

    async fn input_to_output(&self, input: Input) -> Result<Output, TestError> {
        let result = self
            .aggregate
            .call_map(FlatbufferAnnotated::from(input.as_bytes()), &mut None)
            .await?;

        Ok(result)
    }

    async fn output_to_actual_output(&self, output: Output) -> Result<ActualOutput, TestError> {
        let output = output.data();

        let Some(items) = output.items() else {
            return Err(TestError::Unspecified(
                "No AggregateMapMultiOutput::items".to_string(),
            ));
        };

        let mut seq = YamlSequence::with_capacity(items.len());

        let target_key_scalar = Rc::new(YamlNode {
            value: YamlNodeValue::Scalar(YamlScalar {
                value: Rc::from("target_key"),
            }),
            tag: None,
            start_mark: YamlMark::empty(),
        });
        let mapped_value_scalar = Rc::new(YamlNode {
            value: YamlNodeValue::Scalar(YamlScalar {
                value: Rc::from("mapped_value"),
            }),
            tag: None,
            start_mark: YamlMark::empty(),
        });

        let aggregate_hr_instance = self.aggregate_human_readable.instance;

        let input_vec_holder = aggregate_hr_instance.alloc_vec_holder().await?;
        let output_vec_holder = aggregate_hr_instance.alloc_vec_holder().await?;

        for item in items {
            let Some(target_key) = item.target_key() else {
                return Err(TestError::Unspecified(
                    "No AggregateMapOutput::target_key".to_string(),
                ));
            };

            let target_key = call_human_readable_conversion!(
                target_key.bytes(),
                self.aggregate_human_readable,
                call_bytes_to_target_key,
                input_vec_holder,
                output_vec_holder
            )
            .observe_bytes(aggregate_hr_instance, |bytes| {
                Ok::<_, TestError>(String::from(from_utf8(bytes)?))
            })?;

            let mapped_value = item.mapped_value();

            let mapped_value = match mapped_value {
                None => None,
                Some(x) => {
                    let mapped_value = call_human_readable_conversion!(
                        x.bytes(),
                        self.aggregate_human_readable,
                        call_bytes_to_mapped_value,
                        input_vec_holder,
                        output_vec_holder
                    )
                    .observe_bytes(aggregate_hr_instance, |bytes| {
                        Ok::<_, TestError>(String::from(from_utf8(bytes)?))
                    })?;

                    Some(mapped_value)
                }
            };

            let mut mapping =
                YamlMapping::with_capacity(if mapped_value.is_some() { 2 } else { 1 });

            let target_key = Rc::new(YamlNode {
                value: YamlNodeValue::Scalar(YamlScalar {
                    value: Rc::from(target_key),
                }),
                tag: None,
                start_mark: YamlMark::empty(),
            });

            mapping.items.push((target_key_scalar.clone(), target_key));

            if let Some(mapped_value) = mapped_value {
                let mapped_value = Rc::new(YamlNode {
                    value: YamlNodeValue::Scalar(YamlScalar {
                        value: Rc::from(mapped_value),
                    }),
                    tag: None,
                    start_mark: YamlMark::empty(),
                });

                mapping
                    .items
                    .push((mapped_value_scalar.clone(), mapped_value));
            }

            seq.items.push(Rc::new(YamlNode {
                value: YamlNodeValue::Mapping(mapping),
                tag: None,
                start_mark: YamlMark::empty(),
            }));
        }

        let seq = YamlNode {
            value: YamlNodeValue::Sequence(seq),
            tag: None,
            start_mark: YamlMark::empty(),
        };

        let mut result = String::new();

        () = seq.serialize(&mut result)?;

        Ok(result)
    }

    fn expected_output_from_test_vars(
        &self,
        vars: &'a Rc<YamlNode>,
    ) -> Result<ExpectedOutput, TestError> {
        let mut result = String::new();

        () = vars.serialize(&mut result)?;

        Ok(result)
    }

    fn compare_actual_and_expected_output(
        &self,
        actual: &ActualOutput,
        expected: &ExpectedOutput,
    ) -> Result<Option<AssertError>, TestError> {
        Ok(compare_strings(expected, actual))
    }
}

impl<'a> TransformTest<'a> for AggregateMapTransformTest<'a> {
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
