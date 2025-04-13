use crate::collection::constants::{
    COLLECTION_CF_META, COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX,
    COLLECTION_META_PREV_PHANTOM_ID_KEY,
};
use crate::common::{IsByteArray, PhantomId};
use crate::raw_db::{RawDb, RawDbError};
use rocksdb::{AsColumnFamilyRef, WriteBatch, DB};

pub struct StartPhantomOptions<'a> {
    pub phantom_id: PhantomId<'a>,
}

impl RawDb {
    pub fn start_phantom_sync(&self, options: StartPhantomOptions<'_>) -> Result<(), RawDbError> {
        let StartPhantomOptions { phantom_id } = options;

        let db = self.db.get_db();

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        let mut batch = WriteBatch::default();

        // Will write max value
        batch.merge_cf(
            &meta_cf,
            COLLECTION_META_PREV_PHANTOM_ID_KEY,
            phantom_id.get_byte_array(),
        );

        let phantom_key = Self::prefixed_phantom_id_key(phantom_id);

        // Save to drop them at generation finish
        batch.put_cf(&meta_cf, &phantom_key, &[]);

        db.write(batch)?;

        Ok(())
    }

    pub(super) fn prefixed_phantom_id_key(phantom_id: PhantomId) -> Vec<u8> {
        let phantom_id = phantom_id.get_byte_array();
        let mut phantom_key = Vec::with_capacity(
            COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX.len() + phantom_id.len(),
        );
        phantom_key.extend_from_slice(COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX);
        phantom_key.extend_from_slice(phantom_id);

        phantom_key
    }

    pub(super) fn is_phantom_exists(
        db: &DB,
        meta_cf: &impl AsColumnFamilyRef,
        phantom_id: Option<PhantomId<'_>>,
    ) -> Result<bool, RawDbError> {
        let Some(phantom_id) = phantom_id else {
            return Ok(true);
        };

        let pinned = db.get_pinned_cf(meta_cf, &Self::prefixed_phantom_id_key(phantom_id))?;

        Ok(pinned.is_some())
    }
}
