use std::env;
use std::env::VarError;
use std::path::PathBuf;

pub struct Config {
    pub data_path: PathBuf,
}

#[derive(Debug)]
pub enum ReadConfigFromEnvError {
    EnvVarError(env::VarError),
    VarNotPresent(String),
}

impl From<env::VarError> for ReadConfigFromEnvError {
    fn from(err: VarError) -> Self {
        ReadConfigFromEnvError::EnvVarError(err)
    }
}

fn get_var(name: &str) -> Result<String, ReadConfigFromEnvError> {
    let result = env::var("DIFFBELT_DATA_PATH");
    match result {
        Err(err) => match err {
            env::VarError::NotPresent => {
                Err(ReadConfigFromEnvError::VarNotPresent(name.to_string()))
            }
            err => Err(ReadConfigFromEnvError::EnvVarError(err)),
        },
        Ok(value) => Ok(value),
    }
}

impl Config {
    pub fn read_from_env() -> Result<Self, ReadConfigFromEnvError> {
        let data_path = get_var("DIFFBELT_DATA_PATH")?;
        let data_path = PathBuf::from(data_path);

        Ok(Config { data_path })
    }
}
