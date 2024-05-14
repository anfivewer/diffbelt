use crate::collection::Collection;
use crate::common::GenerationId;

impl Collection {
    pub fn generation_is_less_than_minimum(&self, generation_id: GenerationId<'_>) -> bool {
        let minimum_generation_id = self.minimum_generation_id.borrow();
        let minimum_generation_id = minimum_generation_id.as_ref();

        generation_id < minimum_generation_id
    }
}
