use std::env;
use std::env::VarError;
use std::path::PathBuf;

pub struct Config {
    pub data_path: PathBuf,
    pub is_clear: bool,
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
    let result = env::var(name);
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

fn get_opt_var(name: &str) -> Result<Option<String>, ReadConfigFromEnvError> {
    let result = env::var(name);
    match result {
        Err(err) => match err {
            VarError::NotPresent => Ok(None),
            err => Err(ReadConfigFromEnvError::EnvVarError(err)),
        },
        Ok(value) => Ok(Some(value)),
    }
}

impl Config {
    pub fn read_from_env() -> Result<Self, ReadConfigFromEnvError> {
        let data_path = get_var("DIFFBELT_DATA_PATH")?;
        let data_path = PathBuf::from(data_path);

        Ok(Config {
            data_path,
            is_clear: get_opt_var("DIFFBELT_CLEAR")?.unwrap_or("0".to_string()) == "1",
        })
    }
}
