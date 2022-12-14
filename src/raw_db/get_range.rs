use crate::raw_db::{RawDb, RawDbError};
use rocksdb::{Direction, IteratorMode, ReadOptions};

impl RawDb {
    pub async fn get_range_cf(
        &self,
        cf_name: &str,
        from_key: &[u8],
        to_key: &[u8],
    ) -> Result<Vec<(Box<[u8]>, Box<[u8]>)>, RawDbError> {
        let from_key = from_key.to_owned().into_boxed_slice();
        let to_key = to_key.to_owned().into_boxed_slice();

        let db = self.db.clone();
        let cf_name = cf_name.to_string();

        tokio::task::spawn_blocking(move || {
            let db = db.get_db();

            let iterator_mode = IteratorMode::From(&from_key, Direction::Forward);
            let mut opts = ReadOptions::default();
            opts.set_iterate_upper_bound(to_key);

            let iterator = {
                let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
                db.iterator_cf_opt(&cf, opts, iterator_mode)
            };

            let mut result: Vec<(Box<[u8]>, Box<[u8]>)> = Vec::new();

            for item in iterator {
                let item = item?;

                result.push(item);
            }

            Ok(result)
        })
        .await?
    }

    pub fn get_range_sync_cf(
        &self,
        cf_name: &str,
        from_key: &[u8],
        to_key: &[u8],
    ) -> Result<Vec<(Box<[u8]>, Box<[u8]>)>, RawDbError> {
        let from_key = from_key.to_owned().into_boxed_slice();
        let to_key = to_key.to_owned().into_boxed_slice();

        let db = self.db.get_db();

        let iterator_mode = IteratorMode::From(&from_key, Direction::Forward);
        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(to_key);

        let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
        let iterator = db.iterator_cf_opt(&cf, opts, iterator_mode);

        let mut result: Vec<(Box<[u8]>, Box<[u8]>)> = Vec::new();

        for item in iterator {
            let item = item?;

            result.push(item);
        }

        Ok(result)
    }
}
