use crate::collection::constants::COLLECTION_CF_GENERATIONS;
use crate::collection::util::generation_key::OwnedGenerationKey;
use crate::common::{CollectionKey, GenerationId, IsByteArray};
use crate::raw_db::{RawDb, RawDbError};
use rocksdb::{Direction, IteratorMode};

pub struct HasGenerationChangesOptions<'a> {
    pub generation_id: GenerationId<'a>,
}

impl RawDb {
    pub fn has_generation_changes_sync(
        &self,
        options: HasGenerationChangesOptions<'_>,
    ) -> Result<bool, RawDbError> {
        let generation_id = options.generation_id;

        let db = self.db.get_db();

        let generations_cf = db
            .cf_handle(COLLECTION_CF_GENERATIONS)
            .ok_or(RawDbError::CfHandle)?;

        let from_generation_key = OwnedGenerationKey::new(generation_id, CollectionKey::empty())
            .or(Err(RawDbError::InvalidGenerationKey))?;

        let iterator = db.iterator_cf(
            &generations_cf,
            IteratorMode::From(from_generation_key.get_byte_array(), Direction::Forward),
        );

        for item in iterator {
            item?;
            return Ok(true);
        }

        Ok(false)
    }
}
