pub enum FunctionEvalAction {
    MapFilter(MapFilterEvalAction),
}

pub struct MapFilterEvalAction {
    pub source: Box<[u8]>,
}
