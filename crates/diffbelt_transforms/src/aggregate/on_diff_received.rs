use crate::aggregate::AggregateTransform;
use crate::transform::HandlerResult;
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;

impl AggregateTransform {
    pub fn on_diff_received(
        &mut self,
        _diff: DiffCollectionResponseJsonData,
    ) -> HandlerResult<Self> {
        todo!()
    }
}
