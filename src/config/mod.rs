use std::env;
use std::env::VarError;

pub struct Config {
    pub data_path: String,
}

#[derive(Debug)]
pub enum ReadConfigFromEnvError {
    EnvVarError(env::VarError),
}

impl From<env::VarError> for ReadConfigFromEnvError {
    fn from(err: VarError) -> Self {
        ReadConfigFromEnvError::EnvVarError(err)
    }
}

impl Config {
    pub fn read_from_env() -> Result<Self, ReadConfigFromEnvError> {
        let data_path = env::var("DIFFBELT_DATA_PATH")?;

        Ok(Config { data_path })
    }
}
