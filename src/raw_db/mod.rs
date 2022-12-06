use rocksdb::{Error, Options, DB};
use std::borrow::Borrow;
use std::sync::Arc;

pub struct RawDb {
    db: Arc<DB>,
}

impl RawDb {
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

            return Ok(value) as Result<u32, Error>;
        })
        .await
        .or(Err(()))?
        .or(Err(()))
    }
}

pub struct RawDbOptions<'a> {
    pub path: &'a str,
    pub column_families: &'a Vec<&'a str>,
}

pub fn create_raw_db(options: RawDbOptions) -> RawDb {
    let path = options.path;

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    let column_families = options.column_families;

    let db = DB::open_cf(&opts, path, column_families).expect("raw_db, cannot open RocksDB");

    return RawDb { db: Arc::new(db) };
}
