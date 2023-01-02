use crate::raw_db::{RawDb, RawDbError};

impl RawDb {
    pub fn delete_cf_sync(&self, cf_name: &str, key: &[u8]) -> Result<(), RawDbError> {
        let cf = self.db.cf_handle(cf_name).ok_or(RawDbError::CfHandle)?;
        self.db.delete_cf(&cf, key)?;

        Ok(())
    }

    pub async fn delete_cf(&self, cf_name: &str, key: Box<[u8]>) -> Result<(), RawDbError> {
        let db = self.db.clone();
        let cf_name = cf_name.to_string();

        tokio::task::spawn_blocking(move || {
            let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
            db.delete_cf(&cf, key)?;

            Ok(())
        })
        .await?
    }
}
