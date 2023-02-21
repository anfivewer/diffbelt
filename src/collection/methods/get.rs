use crate::collection::methods::errors::CollectionMethodError;

use crate::collection::util::record_key::OwnedRecordKey;
use crate::collection::Collection;

use crate::common::{
    KeyValue, OwnedCollectionKey, OwnedCollectionValue, OwnedGenerationId, OwnedPhantomId,
};

use crate::raw_db::get_collection_record::GetCollectionRecordOptions;

pub struct CollectionGetOptions {
    pub key: OwnedCollectionKey,
    pub generation_id: Option<OwnedGenerationId>,
    pub phantom_id: Option<OwnedPhantomId>,
}

#[derive(Debug)]
pub struct CollectionGetOk {
    pub generation_id: OwnedGenerationId,
    pub item: Option<KeyValue>,
}

impl Collection {
    pub async fn get(
        &self,
        options: CollectionGetOptions,
    ) -> Result<CollectionGetOk, CollectionMethodError> {
        let generation_id = self.generation_id_or_current(options.generation_id).await;

        let record_key = OwnedRecordKey::new(
            options.key.as_ref(),
            generation_id.as_ref(),
            OwnedPhantomId::or_empty_as_ref(&options.phantom_id),
        )
        .or(Err(CollectionMethodError::InvalidKey))?;

        let deletion_lock = self.is_deleted.read().await;
        if deletion_lock.to_owned() {
            return Err(CollectionMethodError::NoSuchCollection);
        }

        let result = self
            .raw_db
            .get_collection_record(GetCollectionRecordOptions {
                record_key: record_key.as_ref(),
            })
            .await?;

        drop(deletion_lock);

        let mut generation_id = generation_id;

        let item: Option<KeyValue> = result.map(
            |(record_key, value): (OwnedRecordKey, OwnedCollectionValue)| {
                let record_key = record_key.as_ref();
                generation_id = record_key.get_generation_id().to_owned();

                KeyValue {
                    key: record_key.get_collection_key().to_owned(),
                    value,
                }
            },
        );

        Ok(CollectionGetOk {
            generation_id,
            item,
        })
    }
}
