use crate::collection::methods::errors::CollectionMethodError;

use crate::collection::util::record_key::OwnedRecordKey;
use crate::collection::Collection;

use crate::common::{
    CollectionKey, CollectionValue, GenerationId, KeyValue, PhantomId,
};


use crate::raw_db::get_collection_record::GetCollectionRecordOptions;



pub struct CollectionGetOptions {
    pub key: CollectionKey,
    pub generation_id: Option<GenerationId>,
    pub phantom_id: Option<PhantomId>,
}

#[derive(Debug)]
pub struct CollectionGetOk {
    pub generation_id: GenerationId,
    pub item: Option<KeyValue>,
}

impl Collection {
    pub async fn get(
        &self,
        options: CollectionGetOptions,
    ) -> Result<CollectionGetOk, CollectionMethodError> {
        let current_generation_id_lock = self.generation_id.read().unwrap();
        let current_generation_id = current_generation_id_lock.clone();
        drop(current_generation_id_lock);

        let generation_id = options.generation_id.unwrap_or(current_generation_id);

        let record_key = OwnedRecordKey::new(
            options.key.as_ref(),
            generation_id.as_ref(),
            PhantomId::or_empty_as_ref(&options.phantom_id),
        )
        .or(Err(CollectionMethodError::InvalidKey))?;

        let result = self
            .raw_db
            .get_collection_record(GetCollectionRecordOptions {
                record_key: record_key.as_ref(),
            })
            .await?;

        let mut generation_id = generation_id;

        let item: Option<KeyValue> =
            result.map(|(record_key, value): (OwnedRecordKey, CollectionValue)| {
                let record_key = record_key.as_ref();
                generation_id = record_key.get_generation_id().to_owned();

                KeyValue {
                    key: record_key.get_key().to_owned(),
                    value,
                }
            });

        Ok(CollectionGetOk {
            generation_id,
            item,
        })
    }
}
