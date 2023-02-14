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
                let current_generation_id_lock = self.generation_id.read().await;
                current_generation_id_lock.clone()
            }
        }
    }
}
