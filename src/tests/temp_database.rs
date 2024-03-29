use crate::database::config::DatabaseConfig;
use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;
use crate::tests::temp_dir::TempDir;
use std::sync::Arc;

pub struct TempDatabase {
    temp_dir: Option<TempDir>,
    database: Option<Database>,
}

impl TempDatabase {
    pub async fn new() -> Self {
        Self::new_with_config(Default::default()).await
    }

    pub async fn new_with_config(config: DatabaseConfig) -> Self {
        let temp_dir = TempDir::new().unwrap();

        println!("Temp dir: {:?}", temp_dir.get_path_buf());

        let database = Database::open(DatabaseOpenOptions {
            data_path: temp_dir.get_path_buf(),
            config: Arc::new(config),
        })
        .await
        .expect("Cannot open database");

        Self {
            temp_dir: Some(temp_dir),
            database: Some(database),
        }
    }

    pub fn get_database(&self) -> &Database {
        self.database.as_ref().unwrap()
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        // Drop database first to close files
        self.database.take();
        // Delete temp directory
        self.temp_dir.take();
    }
}
