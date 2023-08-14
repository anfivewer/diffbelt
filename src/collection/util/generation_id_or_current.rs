use crate::collection::Collection;
use crate::common::OwnedGenerationId;

impl Collection {
    pub async fn generation_id_or_current(
        &self,
        generation_id: Option<OwnedGenerationId>,
    ) -> OwnedGenerationId {
        match generation_id {
            Some(generation_id) => generation_id,
            None => {
                let pair = self.generation_pair_receiver.borrow();
                pair.generation_id.clone()
            }
        }
    }
}
