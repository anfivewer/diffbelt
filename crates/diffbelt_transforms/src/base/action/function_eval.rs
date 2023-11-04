use diffbelt_protos::OwnedSerialized;
use diffbelt_protos::protos::transform::map_filter::MapFilterMultiInput;

#[derive(Debug, Eq, PartialEq)]
pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
}

#[derive(Debug, Eq, PartialEq)]
pub struct MapFilterEvalAction {
    pub input: OwnedSerialized<'static, MapFilterMultiInput<'static>>,
    /// returned back `input` buffer from [`crate::base::input::function_eval::MapFilterEvalInput`]
    pub output_buffer: Vec<u8>,
}
