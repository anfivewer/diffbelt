use diffbelt_protos::OwnedSerialized;
use diffbelt_protos::protos::transform::map_filter::{MapFilterMultiInput, MapFilterMultiOutput};

#[derive(Debug)]
pub struct FunctionEvalInput<T> {
    pub body: T,
}

#[derive(Debug)]
pub enum FunctionEvalInputBody {
    MapFilter(MapFilterEvalInput),
}

#[derive(Debug)]
pub struct MapFilterEvalRecord {
    pub pos: usize,
    pub len: usize,
}

#[derive(Debug)]
pub struct MapFilterEvalInput {
    pub input: OwnedSerialized<'static, MapFilterMultiOutput<'static>>,
    /// returned back `input` buffer from [`crate::base::action::function_eval::MapFilterEvalAction`]
    pub output_buffer: Vec<u8>,
}
