use crate::raw_db::{RawDb, RawDbError};
use rocksdb::Options;

pub struct DestroyOk {
    pub path: String,
}

impl RawDb {
    pub fn destroy(&self) -> Result<DestroyOk, RawDbError> {
        let opts = Options::default();
        rocksdb::DB::destroy(&opts, &self.path).map_err(|err| RawDbError::RocksDb(err))?;

        Ok(DestroyOk {
            path: self.path.clone(),
        })
    }
}
