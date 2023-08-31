use crate::raw_db::RawDb;
use diffbelt_util::debug_print::debug_print;
use std::ops::Deref;
use std::sync::Arc;

pub struct DebugRawDb {
    name: String,
    db: Arc<RawDb>,
}

impl AsRef<RawDb> for DebugRawDb {
    fn as_ref(&self) -> &RawDb {
        self.db.as_ref()
    }
}

impl Deref for DebugRawDb {
    type Target = RawDb;

    fn deref(&self) -> &Self::Target {
        self.db.deref()
    }
}

impl Clone for DebugRawDb {
    fn clone(&self) -> Self {
        debug_print(format!("Clone {} {}", self.name, Arc::strong_count(&self.db)).as_str());

        Self {
            name: self.name.clone(),
            db: self.db.clone(),
        }
    }
}

impl Drop for DebugRawDb {
    fn drop(&mut self) {
        debug_print(format!("Drop {} {}", self.name, Arc::strong_count(&self.db)).as_str());
    }
}

#[cfg(not(feature = "debug_prints"))]
pub type CollectionRawDb = Arc<RawDb>;

#[cfg(feature = "debug_prints")]
pub type CollectionRawDb = DebugRawDb;

pub fn wrap_collection_raw_db(
    db: Arc<RawDb>,
    #[cfg(feature = "debug_prints")] name: String,
) -> CollectionRawDb {
    #[cfg(not(feature = "debug_prints"))]
    {
        return db;
    }

    #[cfg(feature = "debug_prints")]
    DebugRawDb { name, db }
}
