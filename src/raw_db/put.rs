use crate::raw_db::{RawDb, RawDbError};
use rocksdb::WriteBatchWithTransaction;
use std::borrow::Borrow;

pub struct PutKeyValue<'a> {
    pub key: &'a [u8],
    pub value: &'a [u8],
}

impl RawDb {
    pub async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), RawDbError> {
        let key = key.to_owned().into_boxed_slice();
        let value = value.to_owned().into_boxed_slice();

        let db = self.db.clone();
        let cf_name = self.cf_name.clone();

        tokio::task::spawn_blocking(move || {
            match cf_name.borrow() {
                Some(cf_name) => {
                    let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
                    db.put_cf(&cf, key, value)?
                }
                None => db.put(key, value)?,
            };

            Ok(())
        })
        .await?
    }

    pub async fn put_cf(&self, cf_name: &str, key: &[u8], value: &[u8]) -> Result<(), RawDbError> {
        let key = key.to_owned().into_boxed_slice();
        let value = value.to_owned().into_boxed_slice();

        let db = self.db.clone();
        let cf_name = cf_name.to_string();

        tokio::task::spawn_blocking(move || {
            let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
            db.put_cf(&cf, key, value)?;

            Ok(())
        })
        .await?
    }

    pub fn put_cf_sync(
        &self,
        cf_name: &str,
        key: &'_ [u8],
        value: &'_ [u8],
    ) -> Result<(), RawDbError> {
        let cf = self.db.cf_handle(cf_name).ok_or(RawDbError::CfHandle)?;
        self.db.put_cf(&cf, key, value)?;

        Ok(())
    }

    pub fn put_two_sync_cf(
        &self,
        cf_name: &str,
        first: PutKeyValue<'_>,
        second: PutKeyValue<'_>,
    ) -> Result<(), RawDbError> {
        let mut batch = WriteBatchWithTransaction::<false>::default();

        let cf = self.db.cf_handle(cf_name).ok_or(RawDbError::CfHandle)?;

        batch.put_cf(&cf, first.key, first.value);
        batch.put_cf(&cf, second.key, second.value);

        self.db.write(batch)?;

        Ok(())
    }
}
