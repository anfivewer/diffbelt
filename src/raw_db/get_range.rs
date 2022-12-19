use crate::raw_db::{RawDb, RawDbError};
use rocksdb::{Direction, IteratorMode, ReadOptions};

impl RawDb {
    pub async fn get_key_range(
        &self,
        from_key: &[u8],
        to_key: &[u8],
    ) -> Result<Vec<Box<[u8]>>, RawDbError> {
        let from_key = from_key.to_owned().into_boxed_slice();
        let to_key = to_key.to_owned().into_boxed_slice();

        let db = self.db.clone();
        let cf_name = self.cf_name.clone();

        tokio::task::spawn_blocking(move || {
            let iterator_mode = IteratorMode::From(&from_key, Direction::Forward);
            let mut opts = ReadOptions::default();
            opts.set_iterate_upper_bound(to_key);

            let iterator = match cf_name.as_ref() {
                Some(cf_name) => {
                    let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
                    db.iterator_cf_opt(&cf, opts, iterator_mode)
                }
                None => db.iterator_opt(iterator_mode, opts),
            };

            let mut result: Vec<Box<[u8]>> = Vec::new();

            for item in iterator {
                let (key, _) = item?;

                result.push(key);
            }

            Ok(result)
        })
        .await?
    }

    pub async fn get_range(
        &self,
        from_key: &[u8],
        to_key: &[u8],
    ) -> Result<Vec<(Box<[u8]>, Box<[u8]>)>, RawDbError> {
        let from_key = from_key.to_owned().into_boxed_slice();
        let to_key = to_key.to_owned().into_boxed_slice();

        let db = self.db.clone();
        let cf_name = self.cf_name.clone();

        tokio::task::spawn_blocking(move || {
            let iterator_mode = IteratorMode::From(&from_key, Direction::Forward);
            let mut opts = ReadOptions::default();
            opts.set_iterate_upper_bound(to_key);

            let iterator = match cf_name.as_ref() {
                Some(cf_name) => {
                    let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
                    db.iterator_cf_opt(&cf, opts, iterator_mode)
                }
                None => db.iterator_opt(iterator_mode, opts),
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

    pub fn get_range_sync(
        &self,
        from_key: &[u8],
        to_key: &[u8],
    ) -> Result<Vec<(Box<[u8]>, Box<[u8]>)>, RawDbError> {
        let from_key = from_key.to_owned().into_boxed_slice();
        let to_key = to_key.to_owned().into_boxed_slice();

        let db = &self.db;
        let cf_name = self.cf_name.as_ref();

        let iterator_mode = IteratorMode::From(&from_key, Direction::Forward);
        let mut opts = ReadOptions::default();
        opts.set_iterate_upper_bound(to_key);

        let iterator = match cf_name {
            Some(cf_name) => {
                let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
                db.iterator_cf_opt(&cf, opts, iterator_mode)
            }
            None => db.iterator_opt(iterator_mode, opts),
        };

        let mut result: Vec<(Box<[u8]>, Box<[u8]>)> = Vec::new();

        for item in iterator {
            let item = item?;

            result.push(item);
        }

        Ok(result)
    }
}
