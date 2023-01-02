use rocksdb::{ColumnFamilyDescriptor, MergeOperands, Options, DB, DEFAULT_COLUMN_FAMILY_NAME};

use std::cmp::Ordering;
use std::sync::Arc;

pub mod contains_existing_collection_record;
mod cursor_util;
pub mod delete;
pub mod destroy;
pub mod diff_collection_records;
pub mod get_collection_record;
pub mod get_range;
pub mod has_generation_changes;
pub mod put;
pub mod put_collection_record;
pub mod put_many_collection_records;
pub mod query_collection_records;
pub mod remove_all_records_of_generation;
pub mod update_reader;

pub struct RawDb {
    path: String,
    db: Arc<DB>,
}

#[derive(Debug)]
pub enum RawDbError {
    RocksDb(rocksdb::Error),
    Join(tokio::task::JoinError),
    CfHandle,
    InvalidRecordKey,
    InvalidGenerationKey,
    InvalidReaderValue,
    InvalidGenerationId,
    UpdateReader,
    NoSuchReader,
    CursorDidNotFoundRecord,
    DiffNoChangedKeyRecord,
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
    pub async fn get_cf(&self, cf_name: &str, key: &[u8]) -> Result<Option<Box<[u8]>>, RawDbError> {
        let key = key.to_owned().into_boxed_slice();

        let db = self.db.clone();
        let cf_name = cf_name.to_string();

        tokio::task::spawn_blocking(move || {
            let cf = db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
            let value = db.get_cf(&cf, key)?;

            Ok(value.map(|x| x.into_boxed_slice()))
        })
        .await?
    }

    pub fn get_cf_sync(&self, cf_name: &str, key: &[u8]) -> Result<Option<Box<[u8]>>, RawDbError> {
        let cf = self.db.cf_handle(&cf_name).ok_or(RawDbError::CfHandle)?;
        let value = self.db.get_cf(&cf, key)?;

        Ok(value.map(|x| x.into_boxed_slice()))
    }
}

pub struct RawDbComparator {
    pub name: String,
    pub compare_fn: fn(&[u8], &[u8]) -> Ordering,
}

pub struct RawDbMerge {
    pub name: String,
    pub full_merge: Box<
        dyn Fn(&[u8], Option<&[u8]>, &MergeOperands) -> Option<Vec<u8>> + Send + Sync + 'static,
    >,
    pub partial_merge: Box<
        dyn Fn(&[u8], Option<&[u8]>, &MergeOperands) -> Option<Vec<u8>> + Send + Sync + 'static,
    >,
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
            path: path.to_string(),
            db: Arc::new(db),
        });
    }
}
