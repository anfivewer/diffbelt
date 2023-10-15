#[derive(Debug)]
pub struct FunctionEvalInput<T> {
    pub body: T,
}

#[derive(Debug)]
pub enum FunctionEvalInputBody {
    MapFilter(MapFilterEvalInput),
}

#[derive(Debug)]
pub struct MapFilterEvalInput {
    pub old_key: Option<Box<[u8]>>,
    pub new_key: Option<Box<[u8]>>,
    pub value: Option<Box<[u8]>>,
}
