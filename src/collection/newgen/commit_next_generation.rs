use crate::collection::Collection;
use crate::common::{IsByteArray, IsByteArrayMut};
use crate::raw_db::has_generation_changes::HasGenerationChangesOptions;
use crate::raw_db::put::PutKeyValue;
use crate::raw_db::RawDbError;
use crate::util::bytes::increment;
use std::borrow::Borrow;
use std::ops::DerefMut;

impl Collection {
    pub fn commit_next_generation_sync(&self) -> Result<(), RawDbError> {
        // Check that commit is required
        let next_generation_id = {
            let next_generation_id = self.next_generation_id.read().unwrap();
            match next_generation_id.as_ref() {
                Some(next_generation_id) => next_generation_id.as_ref().to_owned(),
                None => {
                    return Ok(());
                }
            }
        };

        let has_changes =
            self.raw_db
                .has_generation_changes_local(HasGenerationChangesOptions {
                    generation_id: next_generation_id.as_ref(),
                })?;

        if !has_changes {
            return Ok(());
        }

        let mut new_next_generation_id = next_generation_id.clone();
        increment(new_next_generation_id.get_byte_array_mut());

        // Store new gens
        self.meta_raw_db.put_two_local(
            PutKeyValue {
                key: b"generation_id",
                value: next_generation_id.get_byte_array(),
            },
            PutKeyValue {
                key: b"next_generation_id",
                value: new_next_generation_id.get_byte_array(),
            },
        )?;

        // Lock next generation first to prevent more puts
        let mut next_generation_id_lock = self.next_generation_id.write().unwrap();
        let mut generation_id_lock = self.generation_id.write().unwrap();

        generation_id_lock.replace(next_generation_id);
        next_generation_id_lock.replace(new_next_generation_id);;

        drop(generation_id_lock);
        drop(next_generation_id_lock);

        Ok(())
    }
}
