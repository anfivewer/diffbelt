use crate::database::open::DatabaseOpenOptions;
use crate::database::Database;
use crate::tests::temp_dir::TempDir;

pub struct TempDatabase {
    temp_dir: TempDir,
    database: Database,
}

impl TempDatabase {
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();

        println!("Temp dir: {:?}", temp_dir.get_path_buf());

        let database = Database::open(DatabaseOpenOptions {
            data_path: temp_dir.get_path_buf(),
        })
        .await
        .expect("Cannot open database");

        Self { temp_dir, database }
    }

    pub fn get_database(&self) -> &Database {
        &self.database
    }
}
