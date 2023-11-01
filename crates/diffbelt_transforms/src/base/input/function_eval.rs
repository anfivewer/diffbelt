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
    pub inputs_buffer: Vec<u8>,
    /// start in `buffer` of `MapFilterMultiInput`
    pub inputs_head: usize,
    pub inputs_len: usize,
    /// returned back `inputs_buffer` from [`crate::base::action::function_eval::MapFilterEvalAction`]
    pub outputs_buffer: Vec<u8>,
}
