#[derive(Debug, Eq, PartialEq)]
pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
}

#[derive(Debug, Eq, PartialEq)]
pub struct MapFilterEvalAction {
    pub key: Box<[u8]>,
    pub from_value: Option<Box<[u8]>>,
    pub to_value: Option<Box<[u8]>>,
}
