use std::borrow::Cow;
use std::rc::Rc;

use enum_dispatch::enum_dispatch;

use diffbelt_yaml::YamlNode;

use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_initial_accumulator::{
    AggregateInitialAccumulatorTransformTest, AggregateInitialAccumulatorTransformTestCreator,
};
use crate::config_tests::transforms::aggregate_map::{
    AggregateMapTransformTest, AggregateMapTransformTestCreator,
};
use crate::config_tests::transforms::map_filter::{
    MapFilterTransformTest, MapFilterTransformTestCreator,
};
use crate::wasm::WasmModuleInstance;
use crate::Collection;
use crate::config_tests::transforms::aggregate_reduce::{AggregateReduceTransformTest, AggregateReduceTransformTestCreator};

pub mod aggregate_initial_accumulator;
pub mod aggregate_map;
pub mod aggregate_reduce;
mod aggregate_util;
pub mod map_filter;

pub struct TransformTestPreCreateOptions<'a, T> {
    pub source_collection: &'a Collection,
    pub target_collection: &'a Collection,
    pub data: T,
}

#[enum_dispatch]
#[allow(async_fn_in_trait)]
pub trait TransformTestCreator<'a>: Sized {
    fn required_wasm_modules(&self) -> Result<Vec<Cow<'a, str>>, TestError>;

    async fn create(
        self,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<TransformTestImpl<'a>, TestError>;
}

#[enum_dispatch(TransformTestCreator)]
pub enum TransformTestCreatorImpl<'a> {
    MapFilter(MapFilterTransformTestCreator<'a>),
    AggregateMap(AggregateMapTransformTestCreator<'a>),
    AggregateInitialAccumulator(AggregateInitialAccumulatorTransformTestCreator<'a>),
    AggregateReduce(AggregateReduceTransformTestCreator<'a>),
}

#[enum_dispatch]
#[allow(async_fn_in_trait)]
pub trait TransformTest<'a>: Sized {
    async fn test(
        &self,
        input: &Rc<YamlNode>,
        expected_output: &Rc<YamlNode>,
    ) -> Result<Option<AssertError>, TestError>;
}

#[enum_dispatch(TransformTest)]
pub enum TransformTestImpl<'a> {
    MapFilter(MapFilterTransformTest<'a>),
    AggregateMap(AggregateMapTransformTest<'a>),
    AggregateInitialAccumulator(AggregateInitialAccumulatorTransformTest<'a>),
    AggregateReduce(AggregateReduceTransformTest<'a>),
}

#[macro_export]
macro_rules! call_human_readable_conversion {
    ($value:expr, $human_readable:expr, $method:ident, $input_vec_holder:ident, $output_vec_holder:ident) => {{
        () = $input_vec_holder.replace_with_slice($value).await?;
        let slice = $human_readable
            .instance
            .vec_to_bytes_slice(&$input_vec_holder)?;
        $human_readable.$method(slice, &$output_vec_holder).await?
    }};
}

#[macro_export]
macro_rules! yaml_test_vars_input_required {
    (
        name: $name:literal,
        value: $value:expr,
        $serializer:ident,
        human_readable: $human_readable:ident.$method:ident,
        $input_vec_holder:ident,
        $output_vec_holder:ident,
        output_offset: $output_offset:ident,
    ) => {{
        let value = $value.ok_or_else(|| {
            crate::config_tests::error::YamlTestVarsError::Unspecified(
                concat!($name, " should be a string").to_string(),
            )
        })?;

        () = call_human_readable_conversion!(
            value.as_bytes(),
            $human_readable,
            $method,
            $input_vec_holder,
            $output_vec_holder
        )
        .observe_bytes($human_readable.instance, |bytes| {
            $output_offset = Some($serializer.create_vector(bytes));

            Ok::<_, crate::config_tests::error::YamlTestVarsError>(())
        })?;
    }};
}

#[macro_export]
macro_rules! yaml_test_vars_input_optional {
    (
        $value:expr,
        $serializer:ident,
        human_readable: $human_readable:ident.$method:ident,
        $input_vec_holder:ident,
        $output_vec_holder:ident,
        output_offset: $output_offset:ident,
    ) => {{
        if let Scalar::String(value) = $value {
            () = call_human_readable_conversion!(
                value.as_bytes(),
                $human_readable,
                $method,
                $input_vec_holder,
                $output_vec_holder
            )
            .observe_bytes($human_readable.instance, |bytes| {
                $output_offset = Some($serializer.create_vector(bytes));

                Ok::<_, crate::config_tests::error::YamlTestVarsError>(())
            })?;
        }
    }};
}
