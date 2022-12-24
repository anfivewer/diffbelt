use crate::collection::util::generation_key::{GenerationKey, OwnedGenerationKey};
use crate::common::{CollectionKey, GenerationId, IsByteArray, IsByteArrayMut, OwnedCollectionKey};
use crate::raw_db::diff_collection_records::state::{
    DiffState, DiffStateInMemoryMode, DiffStateMode,
};
use crate::raw_db::diff_collection_records::DiffCollectionRecordsResult;
use crate::raw_db::RawDbError;
use crate::util::bytes::increment;
use rocksdb::{Direction, IteratorMode, ReadOptions};

pub struct SingleGenerationChangedKeysIter<'a> {
    iterator: rocksdb::DBIterator<'a>,
}

impl<'a> SingleGenerationChangedKeysIter<'a> {
    pub fn new(
        db: &'a rocksdb::DB,
        generation_id: GenerationId<'_>,
        from_collection_key: Option<CollectionKey<'_>>,
    ) -> Result<Self, RawDbError> {
        let generations_cf = db.cf_handle("gens").ok_or(RawDbError::CfHandle)?;

        let iterator = {
            let from_generation_key = OwnedGenerationKey::new(
                generation_id,
                CollectionKey::or_empty(&from_collection_key),
            )
            .or(Err(RawDbError::InvalidGenerationKey))?;

            let to_generation_key = {
                let mut to_generation_id_incremented = generation_id.to_owned();

                let to_generation_id_bytes = to_generation_id_incremented.get_byte_array_mut();
                increment(to_generation_id_bytes);
                OwnedGenerationKey::new(
                    to_generation_id_incremented.as_ref(),
                    CollectionKey::empty(),
                )
                .or(Err(RawDbError::InvalidGenerationKey))?
            };

            let iterator_mode =
                IteratorMode::From(from_generation_key.get_byte_array(), Direction::Forward);
            let mut opts = ReadOptions::default();
            opts.set_iterate_upper_bound(to_generation_key.get_byte_array());
            db.iterator_cf_opt(&generations_cf, opts, iterator_mode)
        };

        Ok(Self { iterator })
    }
}

impl Iterator for SingleGenerationChangedKeysIter<'_> {
    type Item = Result<OwnedCollectionKey, RawDbError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|result| {
            let (key, _): (Box<[u8]>, Box<[u8]>) = result?;
            let generation_key =
                GenerationKey::validate(&key).or(Err(RawDbError::InvalidGenerationKey))?;
            Ok(generation_key.get_collection_key().to_owned())
        })
    }
}
