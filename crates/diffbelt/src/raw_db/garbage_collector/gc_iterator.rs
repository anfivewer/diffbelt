use crate::collection::util::record_key::{OwnedRecordKey, ParsedRecordKey, RecordKey};
use rocksdb::{DBRawIterator, ReadOptions};

use crate::common::PhantomId;
use crate::raw_db::RawDbError;

pub struct GcIterator<'a> {
    is_first: bool,
    gc_phantom_id: Option<PhantomId<'a>>,
    db_iterator: DBRawIterator<'a>,
    keys_to_delete: &'a mut Vec<OwnedRecordKey>,

    is_saved: bool,
    saved_key: Vec<u8>,
    was_first: bool,
}

pub struct NewGcIterator<'a> {
    pub gc_phantom_id: Option<PhantomId<'a>>,
    pub db_iterator: DBRawIterator<'a>,
    pub keys_to_delete: &'a mut Vec<OwnedRecordKey>,
}

impl<'a> NewGcIterator<'a> {
    pub fn new(self) -> GcIterator<'a> {
        GcIterator {
            is_first: true,
            gc_phantom_id: self.gc_phantom_id,
            db_iterator: self.db_iterator,
            keys_to_delete: self.keys_to_delete,
            is_saved: false,
            saved_key: Vec::new(),
            was_first: true,
        }
    }
}

struct Iter<'a> {
    iterator: &'a mut GcIterator<'a>,
}

impl<'a> GcIterator<'a> {
    pub fn next(&mut self) -> Result<Option<(ParsedRecordKey, &[u8])>, RawDbError> {
        if self.is_first {
            self.is_first = false;
        } else {
            self.db_iterator.next();
            () = self.db_iterator.status()?;
        }

        let Some(gc_phantom_id) = self.gc_phantom_id else {
            let Some((key, value)) = self.db_iterator.item() else {
                return Ok(None);
            };

            let record_key = RecordKey::validate(key).map_err(|()| RawDbError::InvalidRecordKey)?;
            let record_key = record_key.parse();

            return Ok(Some((record_key, value)));
        };

        let db_iterator_ptr = &mut self.db_iterator as *mut DBRawIterator;

        loop {
            let db_iterator = unsafe { &*db_iterator_ptr };
            let Some((key, value)) = db_iterator.item() else {
                return Ok(None);
            };

            let record_key = RecordKey::validate(key).map_err(|()| RawDbError::InvalidRecordKey)?;
            let parsed_record_key = record_key.parse();

            if is_need_to_delete(parsed_record_key.phantom_id, gc_phantom_id) {
                self.keys_to_delete.push(record_key.to_owned());

                let db_iterator = unsafe { &mut *db_iterator_ptr };
                db_iterator.next();
                () = db_iterator.status()?;
                continue;
            }

            return Ok(Some((parsed_record_key, value)));
        }
    }

    pub fn save_state(&mut self) -> Result<(), RawDbError> {
        if self.is_saved {
            panic!("Cannot save state twice");
        }

        self.is_saved = true;
        self.was_first = self.is_first;

        if self.was_first {
            return Ok(());
        }

        let Some(key) = self.db_iterator.key() else {
            self.was_first = true;
            return Ok(());
        };

        self.saved_key.clear();
        self.saved_key.reserve(key.len());
        self.saved_key.extend_from_slice(key);

        Ok(())
    }

    pub fn restore_state(&mut self) -> Result<(), RawDbError> {
        if !self.is_saved {
            panic!("Was not saved");
        }

        self.is_saved = false;
        self.is_first = self.was_first;

        if self.is_first {
            self.db_iterator.seek_to_first();
        } else {
            self.db_iterator.seek(&self.saved_key);
        }

        () = self.db_iterator.status()?;
        Ok(())
    }

    pub fn into_keys_to_delete(self) -> &'a mut Vec<OwnedRecordKey> {
        self.keys_to_delete
    }
}

fn is_need_to_delete(phantom_id: Option<PhantomId<'_>>, gc_phantom_id: PhantomId<'_>) -> bool {
    if let Some(phantom_id) = phantom_id {
        if phantom_id <= gc_phantom_id {
            return true;
        }
    }

    false
}
