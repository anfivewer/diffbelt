use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::transforms::aggregate_map::{
    AggregateMapTransformTest, AggregateMapTransformTestCreator,
};
use crate::config_tests::transforms::map_filter::{
    MapFilterTransformTest, MapFilterTransformTestCreator,
};
use crate::wasm::WasmModuleInstance;
use crate::Collection;
use diffbelt_yaml::YamlNode;
use enum_dispatch::enum_dispatch;
use std::borrow::Cow;
use std::rc::Rc;

pub mod aggregate_map;
pub mod map_filter;

pub struct TransformTestPreCreateOptions<'a, T> {
    pub source_collection: &'a Collection,
    pub target_collection: &'a Collection,
    pub data: T,
}

#[enum_dispatch]
pub trait TransformTestCreator<'a>: Sized {
    fn required_wasm_modules(&self) -> Result<Vec<Cow<'a, str>>, TestError>;

    fn create(
        self,
        wasm_modules: Vec<&'a WasmModuleInstance>,
    ) -> Result<TransformTestImpl<'a>, TestError>;
}

#[enum_dispatch(TransformTestCreator)]
pub enum TransformTestCreatorImpl<'a> {
    MapFilter(MapFilterTransformTestCreator<'a>),
    AggregateMap(AggregateMapTransformTestCreator<'a>),
}

#[enum_dispatch]
pub trait TransformTest<'a>: Sized {
    fn test(
        &self,
        input: &Rc<YamlNode>,
        expected_output: &Rc<YamlNode>,
    ) -> Result<Option<AssertError>, TestError>;
}

#[enum_dispatch(TransformTest)]
pub enum TransformTestImpl<'a> {
    MapFilter(MapFilterTransformTest<'a>),
    AggregateMap(AggregateMapTransformTest<'a>),
}

#[macro_export]
macro_rules! call_human_readable_conversion {
    ($value:ident, $human_readable:ident, $method:ident, $input_vec_holder:ident, $output_vec_holder:ident) => {{
        () = $input_vec_holder.replace_with_slice($value.as_bytes())?;
        let slice = $human_readable
            .instance
            .vec_to_bytes_slice(&$input_vec_holder)?;

        () = $human_readable.$method(&slice.0, &$output_vec_holder)?;

        $output_vec_holder.access()?
    }};
}
