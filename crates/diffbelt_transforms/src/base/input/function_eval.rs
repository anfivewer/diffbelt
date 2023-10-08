pub struct FunctionEvalInput<T> {
    pub body: T,
}

pub enum FunctionEvalInputBody {
    MapFilter(MapFilterEvalInput),
}

pub struct MapFilterEvalInput {
    pub old_key: Option<Box<[u8]>>,
    pub new_key: Option<Box<[u8]>>,
    pub value: Option<Box<[u8]>>,
}
