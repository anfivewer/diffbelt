use rocksdb::{Error, DB};
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
                None => {
                    0 as u32
                }
            };

            let new_value = if value >= u32::MAX {
                0 as u32
            } else {
                value + 1
            };

            let arr = u32::to_be_bytes(new_value);
            db.put("some_key", arr)?;

            return Ok(value) as Result<u32, Error>
        })
        .await
        .or(Err(()))?
        .or(Err(()))
    }
}

pub fn create_raw_db() -> RawDb {
    let db = DB::open_default("/tmp/raw_db").unwrap();

    return RawDb { db: Arc::new(db) };
}
