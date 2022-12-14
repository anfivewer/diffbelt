use rocksdb::merge_operator::MergeFn;
use rocksdb::{
    ColumnFamilyDescriptor, Direction, IteratorMode, Options, ReadOptions, DB,
    DEFAULT_COLUMN_FAMILY_NAME,
};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::sync::Arc;

pub mod contains_existing_collection_record;
pub mod get_collection_record;
pub mod has_generation_changes;
pub mod put;
pub mod put_collection_record;
pub mod put_many_collection_records;
pub mod remove_all_records_of_generation;
pub mod update_reader;

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
    InvalidGenerationKey,
    InvalidReaderValue,
    UpdateReader,
    NoSuchReader,
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

pub struct RawDbMerge {
    pub name: String,
    pub full_merge: Box<dyn MergeFn>,
    pub partial_merge: Box<dyn MergeFn>,
}

pub struct RawDbColumnFamily {
    pub name: String,
    pub comparator: Option<RawDbComparator>,
    pub merge: Option<RawDbMerge>,
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

        let mut column_family_descriptors: Vec<ColumnFamilyDescriptor> =
            Vec::with_capacity(options.column_families.len() + 1);

        let mut default_cf_opts = Options::default();
        match options.comparator {
            Some(comparator) => {
                default_cf_opts.set_comparator(&comparator.name, comparator.compare_fn)
            }
            None => (),
        }
        column_family_descriptors.push(ColumnFamilyDescriptor::new(
            DEFAULT_COLUMN_FAMILY_NAME,
            default_cf_opts,
        ));

        for family in options.column_families {
            let mut cf_opts = Options::default();

            family.comparator.as_ref().map(|comparator| {
                cf_opts.set_comparator(&comparator.name, comparator.compare_fn);
            });

            family.merge.map(|merge| {
                cf_opts.set_merge_operator(&merge.name, merge.full_merge, merge.partial_merge);
            });

            column_family_descriptors.push(ColumnFamilyDescriptor::new(&family.name, cf_opts));
        }

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
