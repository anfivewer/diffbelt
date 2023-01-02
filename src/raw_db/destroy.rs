use crate::raw_db::{RawDb, RawDbError};
use rocksdb::Options;

pub struct DestroyOk {
    pub path: String,
}

impl RawDb {
    pub fn destroy(path: &str) -> Result<(), RawDbError> {
        let opts = Options::default();
        rocksdb::DB::destroy(&opts, path).map_err(|err| RawDbError::RocksDb(err))?;

        Ok(())
    }
}
