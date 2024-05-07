use std::iter;
use std::ops::Deref;

use rocksdb::{AsColumnFamilyRef, DBRawIterator, ReadOptions, WriteBatch, DB};

use crate::collection::constants::{
    COLLECTION_CF_META, COLLECTION_META_GC_PHANTOM_ID_KEY,
    COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX,
    COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX_END,
};
use crate::common::{GenerationId, IsByteArray};
use crate::raw_db::update_reader::RawDbUpdateReaderOptions;
use crate::raw_db::{RawDb, RawDbError};

pub struct RawDbUpdateReader<'a> {
    pub reader_name: &'a str,
    pub generation_id: GenerationId<'a>,
}

pub struct RawDbCommitGenerationOptions<'a> {
    pub generation_id: GenerationId<'a>,
    pub next_generation_id: GenerationId<'a>,
    pub update_readers: Option<Vec<RawDbUpdateReader<'a>>>,
}

impl RawDb {
    pub fn commit_generation_sync(
        &self,
        options: RawDbCommitGenerationOptions<'_>,
    ) -> Result<(), RawDbError> {
        let RawDbCommitGenerationOptions {
            generation_id,
            next_generation_id,
            update_readers,
        } = options;

        let mut batch = WriteBatch::default();

        let db = self.db.get_db();

        let meta_cf = db
            .cf_handle(COLLECTION_CF_META)
            .ok_or(RawDbError::CfHandle)?;

        batch.put_cf(&meta_cf, b"generation_id", generation_id.get_byte_array());
        batch.put_cf(
            &meta_cf,
            b"next_generation_id",
            next_generation_id.get_byte_array(),
        );

        if let Some(update_readers) = update_readers {
            for update in update_readers {
                let RawDbUpdateReader {
                    reader_name,
                    generation_id,
                } = update;

                self.update_reader_batch(
                    &mut batch,
                    meta_cf.clone(),
                    RawDbUpdateReaderOptions {
                        reader_name,
                        generation_id,
                    },
                )?;
            }
        }

        () = Self::on_generation_finish(db, &meta_cf, &mut batch)?;

        db.write(batch)?;

        Ok(())
    }

    pub(super) fn on_generation_finish(
        db: &DB,
        meta_cf: &impl AsColumnFamilyRef,
        batch: &mut WriteBatch,
    ) -> Result<(), RawDbError> {
        // Schedule phantoms cleanup
        let mut opts = ReadOptions::default();
        opts.set_iterate_lower_bound(COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX);
        opts.set_iterate_upper_bound(COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX_END);

        let mut iterator = db.raw_iterator_cf_opt(meta_cf, opts);

        iterator.seek_to_first();

        let mut max_phantom_id: Option<Vec<u8>> = None;

        while let Some(phantom_id) = iterator.key() {
            batch.delete_cf(meta_cf, phantom_id);

            let phantom_id =
                &phantom_id[COLLECTION_META_NEXT_GENERATION_PHANTOMS_KEY_PREFIX.len()..];

            if let Some(max_phantom_id) = max_phantom_id.as_mut() {
                if phantom_id > AsRef::<[u8]>::as_ref(max_phantom_id) {
                    max_phantom_id.clear();
                    max_phantom_id.extend_from_slice(phantom_id);
                }
            } else {
                max_phantom_id = Some(phantom_id.to_vec());
            }

            iterator.next();
        }

        if let Some(max_phantom_id) = max_phantom_id {
            // All phantoms less than this will be deleted when seen
            batch.put_cf(meta_cf, COLLECTION_META_GC_PHANTOM_ID_KEY, &max_phantom_id);
        }

        Ok(())
    }
}
