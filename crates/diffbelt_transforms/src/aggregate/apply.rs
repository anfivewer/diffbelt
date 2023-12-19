use crate::aggregate::context::HandlerContext;
use crate::aggregate::limits::Limits;
use crate::aggregate::state::{ProcessingState, Target, TargetKeyData};
use crate::aggregate::AggregateTransform;
use crate::transform::ActionInputHandlerActionsVec;
use lru::LruCache;
use std::rc::Rc;
use diffbelt_util_no_std::temporary_collection::mutable::vec::TemporaryMutRefVec;

impl AggregateTransform {
    pub fn try_apply(
        actions: &mut ActionInputHandlerActionsVec<Self, HandlerContext>,
        max_limits: &Limits,
        current_limits: &Limits,
        target_keys: &mut LruCache<Rc<[u8]>, Target>,
        temp_targets: &mut TemporaryMutRefVec<Target>,
    ) {
        if !Self::can_eval_apply(max_limits, current_limits) {
            return;
        }

        let is_needed = Self::need_eval_apply(max_limits, current_limits);

        let mut targets_to_apply_temp = temp_targets.temp();
        let target_to_apply = targets_to_apply_temp.as_mut();

        if !is_needed {
            // Finish of diffs, apply all what we can
            for (key, target) in target_keys.iter_mut() {
                let Some(data) = target.as_processing() else {
                    continue;
                };

                // just for test
                target_to_apply.push(target);
            }
        }

        // for (key, target) in target_keys.iter_mut().rev() {
        //     let Some(target) = target.as_processing() {
        //         //
        //     }
        // }

        for target in target_to_apply {
            println!("target {target:?}");
        }

        todo!()
    }
}
