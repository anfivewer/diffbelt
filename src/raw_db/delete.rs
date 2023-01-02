use crate::raw_db::{RawDb, RawDbError};

impl RawDb {
    pub fn delete_sync(&self, key: &[u8]) -> Result<(), RawDbError> {
        let cf_name = self.cf_name.as_ref();

        match cf_name {
            Some(cf_name) => {
                let cf = self.db.cf_handle(cf_name).ok_or(RawDbError::CfHandle)?;
                self.db.delete_cf(&cf, key)?;
            }
            None => {
                self.db.delete(key)?;
            }
        }

        Ok(())
    }

    pub async fn delete(&self, key: Box<[u8]>) -> Result<(), RawDbError> {
        let db = self.db.clone();
        let cf_name = self.cf_name.clone();

        tokio::task::spawn_blocking(move || {
            match cf_name.as_ref() {
                Some(cf_name) => {
                    let cf = db.cf_handle(cf_name).ok_or(RawDbError::CfHandle)?;
                    db.delete_cf(&cf, key)?;
                }
                None => {
                    db.delete(key)?;
                }
            }

            Ok(())
        })
        .await?
    }
}
