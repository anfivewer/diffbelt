use diffbelt_types::collection::get_record::GetResponseJsonData;
use crate::aggregate::AggregateTransform;
use crate::aggregate::context::{HandlerContext, TargetRecordContext};
use crate::transform::HandlerResult;

impl AggregateTransform {
    pub fn on_target_record_received(
        &mut self,
        ctx: TargetRecordContext,
        body: GetResponseJsonData,
    ) -> HandlerResult<Self, HandlerContext> {
        todo!()
    }
}