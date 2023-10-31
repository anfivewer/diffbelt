#[derive(Debug, Eq, PartialEq)]
pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
}

#[derive(Debug, Eq, PartialEq)]
pub struct MapFilterEvalAction {
    pub source_key: Box<[u8]>,
    pub source_old_value: Option<Box<[u8]>>,
    pub source_new_value: Option<Box<[u8]>>,
}
