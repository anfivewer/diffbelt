use crate::raw_db::{RawDb, RawDbError};

impl RawDb {
    pub fn delete_cf_sync(&self, cf_name: &str, key: &[u8]) -> Result<(), RawDbError> {
        let db = self.db.get_db();

        let cf = db.cf_handle(cf_name).ok_or(RawDbError::CfHandle)?;
        db.delete_cf(&cf, key)?;

        Ok(())
    }
}
