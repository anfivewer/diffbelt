use crate::collection::util::record_key::{
    OwnedRecordKey, ParsedRecordKey, ParsedRecordKeyOld, RecordKey,
};
use rocksdb::{DBRawIterator, ReadOptions};

use crate::common::{CollectionValue, IsByteArray, PhantomId};
use crate::raw_db::{RawDb, RawDbError};

pub struct GcIterator<'a> {
    need_seek_to_first: bool,
    need_go_next: bool,
    is_invalid: bool,
    gc_phantom_id: Option<PhantomId<'a>>,
    db_iterator: DBRawIterator<'a>,
    keys_to_delete: &'a mut Vec<OwnedRecordKey>,

    is_saved: bool,
    saved_key: Vec<u8>,
    was_first: bool,
    was_invalid: bool,
}

pub struct NewGcIterator<'a> {
    pub gc_phantom_id: Option<PhantomId<'a>>,
    pub db_iterator: DBRawIterator<'a>,
    pub keys_to_delete: &'a mut Vec<OwnedRecordKey>,
}

impl<'a> NewGcIterator<'a> {
    pub fn new(self) -> GcIterator<'a> {
        GcIterator {
            need_seek_to_first: true,
            need_go_next: false,
            is_invalid: false,
            gc_phantom_id: self.gc_phantom_id,
            db_iterator: self.db_iterator,
            keys_to_delete: self.keys_to_delete,
            is_saved: false,
            saved_key: Vec::new(),
            was_first: true,
            was_invalid: false,
        }
    }
}

struct Iter<'a> {
    iterator: &'a mut GcIterator<'a>,
}

impl<'a> GcIterator<'a> {
    pub fn next(&mut self) -> Result<Option<(ParsedRecordKeyOld, &[u8])>, RawDbError> {
        if self.is_invalid {
            return Ok(None);
        }

        if self.need_seek_to_first {
            self.need_seek_to_first = false;
            self.db_iterator.seek_to_first();
        } else {
            self.db_iterator.next();
            () = self.db_iterator.status()?;
        }

        let Some(gc_phantom_id) = self.gc_phantom_id else {
            let Some((key, value)) = self.db_iterator.item() else {
                return Ok(None);
            };

            let record_key = RecordKey::validate(key).map_err(|()| RawDbError::InvalidRecordKey)?;
            let record_key = record_key.parse_old();

            return Ok(Some((record_key, value)));
        };

        let db_iterator_ptr = &mut self.db_iterator as *mut DBRawIterator;

        loop {
            let db_iterator = unsafe { &*db_iterator_ptr };
            let Some((key, value)) = db_iterator.item() else {
                return Ok(None);
            };

            let record_key = RecordKey::validate(key).map_err(|()| RawDbError::InvalidRecordKey)?;
            let parsed_record_key = record_key.parse_old();

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

    pub fn get_value_for_key(
        &mut self,
        key: RecordKey<'_>,
    ) -> Result<CollectionValue<'_>, RawDbError> {
        let key_bytes = key.get_byte_array();

        () = self.seek(key_bytes)?;

        let Some(actual_key) = self.db_iterator.key() else {
            return Err(RawDbError::Unspecified(
                "get_value_for_key: key not found".to_string(),
            ));
        };

        if actual_key != key_bytes {
            return Err(RawDbError::Unspecified(
                "get_value_for_key: key not matches".to_string(),
            ));
        }

        let Some(value) = self.db_iterator.value() else {
            return Err(RawDbError::Unspecified(
                "get_value_for_key: no value".to_string(),
            ));
        };

        Ok(CollectionValue::from_slice(value))
    }

    pub fn next_key(&mut self) -> Result<Option<ParsedRecordKey>, RawDbError> {
        if self.is_invalid {
            return Ok(None);
        }

        if self.need_seek_to_first {
            self.need_seek_to_first = false;
            self.db_iterator.seek_to_first();
        } else if self.need_go_next {
            self.db_iterator.next();
            () = self.db_iterator.status()?;
        }

        self.need_go_next = true;

        let Some(gc_phantom_id) = self.gc_phantom_id else {
            let Some(key) = self.db_iterator.key() else {
                return Ok(None);
            };

            let record_key = RecordKey::validate(key).map_err(|()| RawDbError::InvalidRecordKey)?;
            let record_key = record_key.parse();

            return Ok(Some(record_key));
        };

        let db_iterator_ptr = &mut self.db_iterator as *mut DBRawIterator;

        loop {
            let db_iterator = unsafe { &*db_iterator_ptr };
            let Some(key) = db_iterator.key() else {
                return Ok(None);
            };

            let record_key = RecordKey::validate(key).map_err(|()| RawDbError::InvalidRecordKey)?;
            let parsed_record_key = record_key.parse();

            if is_need_to_delete(parsed_record_key.phantom_id(), gc_phantom_id) {
                self.keys_to_delete.push(record_key.to_owned());

                let db_iterator = unsafe { &mut *db_iterator_ptr };
                db_iterator.next();
                () = db_iterator.status()?;
                continue;
            }

            return Ok(Some(parsed_record_key));
        }
    }

    pub fn seek(&mut self, key: &[u8]) -> Result<(), RawDbError> {
        self.is_invalid = false;
        self.need_seek_to_first = false;
        self.need_go_next = false;
        self.db_iterator.seek(key);
        () = self.db_iterator.status()?;
        Ok(())
    }

    pub fn make_invalid(&mut self) {
        self.is_invalid = true;
    }

    pub fn save_state(&mut self) -> Result<(), RawDbError> {
        if self.is_saved {
            panic!("Cannot save state twice");
        }

        self.is_saved = true;
        self.was_first = self.need_seek_to_first;

        if self.was_first {
            return Ok(());
        }

        if !self.db_iterator.valid() {
            self.was_invalid = true;
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
        self.need_seek_to_first = self.was_first;
        self.is_invalid = self.was_invalid;

        if self.is_invalid {
            return Ok(());
        }

        if !self.need_seek_to_first {
            self.db_iterator.seek(&self.saved_key);
            self.need_go_next = true;
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
