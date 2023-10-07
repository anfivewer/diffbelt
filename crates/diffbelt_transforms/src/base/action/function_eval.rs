pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
}

pub struct MapFilterEvalAction {
    pub key: Box<[u8]>,
    pub from_value: Option<Box<[u8]>>,
    pub to_value: Option<Box<[u8]>>,
}
