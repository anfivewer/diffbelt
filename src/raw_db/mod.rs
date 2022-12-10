use rocksdb::{ColumnFamilyDescriptor, Direction, IteratorMode, Options, ReadOptions, DB};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::sync::Arc;

pub mod contains_existing_collection_record;

pub struct RawDb {
    db: Arc<DB>,
    cf_name: Arc<Option<String>>,
}

#[derive(Debug)]
pub enum RawDbError {
    RocksDb(rocksdb::Error),
    Join(tokio::task::JoinError),
    CfHandle,
    InvalidRecordKey,
}

impl From<rocksdb::Error> for RawDbError {
    fn from(err: rocksdb::Error) -> Self {
        RawDbError::RocksDb(err)
    }
}

impl From<tokio::task::JoinError> for RawDbError {
    fn from(err: tokio::task::JoinError) -> Self {
        RawDbError::Join(err)
    }
}

impl RawDb {
    pub async fn get(&self, key: &[u8]) -> Result<Option<Box<[u8]>>, RawDbError> {
        let key = key.to_owned().into_boxed_slice();

        let db = self.db.clone();
        let cf_name = self.cf_name.clone();

        tokio::task::spawn_blocking(move || {
            let value = match cf_name.borrow() {
                Some(cf_name) => {
                    let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
                    db.get_cf(&cf, key)?
                }
                None => db.get(key)?,
            };

            Ok(value.map(|x| x.into_boxed_slice()))
        })
        .await?
    }

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

            let iterator = match cf_name.borrow() {
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

            let iterator = match cf_name.borrow() {
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

    pub async fn next_value(&self) -> Result<u32, ()> {
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || {
            let value = db.get("some_key")?;

            let value = match value {
                Some(bytes) => {
                    if bytes.len() != 4 {
                        return Ok(0);
                    }

                    let mut arr = [0u8; 4];
                    arr.clone_from_slice(bytes.as_slice());
                    u32::from_be_bytes(arr)
                }
                None => 0 as u32,
            };

            let new_value = if value >= u32::MAX {
                0 as u32
            } else {
                value + 1
            };

            let arr = u32::to_be_bytes(new_value);
            db.put("some_key", arr)?;

            return Ok(value) as Result<u32, rocksdb::Error>;
        })
        .await
        .or(Err(()))?
        .or(Err(()))
    }
}

pub struct RawDbComparator {
    pub name: String,
    pub compare_fn: fn(&[u8], &[u8]) -> Ordering,
}

pub struct RawDbColumnFamily {
    pub name: String,
    pub comparator: Option<RawDbComparator>,
}

pub struct RawDbOptions<'a> {
    pub path: &'a str,
    pub comparator: Option<RawDbComparator>,
    pub column_families: Vec<RawDbColumnFamily>,
}

#[derive(Debug)]
pub enum RawDbOpenError {
    RocksDbError(rocksdb::Error),
}

impl From<rocksdb::Error> for RawDbOpenError {
    fn from(err: rocksdb::Error) -> Self {
        RawDbOpenError::RocksDbError(err)
    }
}

impl RawDb {
    pub fn open_raw_db(options: RawDbOptions) -> Result<RawDb, RawDbOpenError> {
        let path = options.path;

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        match options.comparator {
            Some(comparator) => opts.set_comparator(&comparator.name, comparator.compare_fn),
            None => (),
        }

        let column_family_descriptors: Vec<ColumnFamilyDescriptor> = options
            .column_families
            .into_iter()
            .map(|family| {
                let mut cf_opts = Options::default();

                family.comparator.as_ref().map(|comparator| {
                    cf_opts.set_comparator(&comparator.name, comparator.compare_fn);
                });

                ColumnFamilyDescriptor::new(&family.name, cf_opts)
            })
            .collect();

        let db = DB::open_cf_descriptors(&opts, path, column_family_descriptors)?;

        return Ok(RawDb {
            db: Arc::new(db),
            cf_name: Arc::new(None),
        });
    }

    pub fn with_cf<S: Into<String>>(&self, name: S) -> Self {
        RawDb {
            db: self.db.clone(),
            cf_name: Arc::new(Some(name.into())),
        }
    }
}
