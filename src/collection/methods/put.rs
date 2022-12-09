use crate::collection::methods::errors::CollectionMethodError;
use crate::collection::Collection;
use crate::common::util::is_byte_array_equal_both_opt;
use crate::common::{GenerationId, KeyValueUpdate, PhantomId};

pub struct CollectionPutOptions {
    update: KeyValueUpdate,
    generation_id: Option<GenerationId>,
    phantom_id: Option<PhantomId>,
}

impl Collection {
    pub async fn put(
        &mut self,
        options: CollectionPutOptions,
    ) -> Result<(), CollectionMethodError> {
        //// Validate request
        let update = options.update;
        let generation_id = options.generation_id;
        let phantom_id = options.phantom_id;

        if phantom_id.is_some() && generation_id.is_none() {
            // Phantom writes can be only to the specified generation
            return Err(CollectionMethodError::PutPhantomWithoutGenerationId);
        }

        let next_generation = self.next_generation.read().await;
        let next_generation_id = next_generation.as_ref().map(|gen| &gen.id);

        let is_generation_id_equal_to_next_one =
            is_byte_array_equal_both_opt(generation_id.as_ref(), next_generation_id);

        // Phantom puts are allowed to do everything (except to be without a specified generationId),
        // but we are already checked it above
        if phantom_id.is_none() {
            if generation_id.is_some() {
                if !is_generation_id_equal_to_next_one {
                    return Err(CollectionMethodError::OutdatedGeneration);
                }
            } else if self.is_manual {
                // we cannot put values is manual collection without specified generationId
                return Err(CollectionMethodError::CannotPutInManualCollection);
            } else if next_generation.is_none() {
                panic!("Collection::put, no next_generation in !manual collection");
            }
        }

        //// Insert
        let record_generation_id = generation_id
            .as_ref()
            .or(next_generation_id)
            .expect("Collection::put, no either generation_id or next_generation");

        todo!();

        Ok(())
    }
}
