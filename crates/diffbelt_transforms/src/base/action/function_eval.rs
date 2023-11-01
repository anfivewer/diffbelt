#[derive(Debug, Eq, PartialEq)]
pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
}

#[derive(Debug, Eq, PartialEq)]
pub struct MapFilterEvalAction {
    pub inputs_buffer: Vec<u8>,
    /// start in `outputs_buffer` of `MapFilterMultiOutput`
    pub inputs_head: usize,
    pub inputs_len: usize,
    /// returned back `inputs_buffer` from [`crate::base::input::function_eval::MapFilterEvalInput`]
    pub outputs_buffer: Vec<u8>,
}
