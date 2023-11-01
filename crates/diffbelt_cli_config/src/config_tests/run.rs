use crate::config_tests::{SingleTestResult, TestResult};
use crate::errors::ConfigParsingError;
use crate::CliConfig;
use diffbelt_yaml::YamlParsingError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RunTestsError {
    #[error(transparent)]
    YamlParsing(YamlParsingError),
    #[error(transparent)]
    ConfigParsing(ConfigParsingError),
}

pub async fn run_tests(config: &CliConfig) -> Result<bool, RunTestsError> {
    let results = config.run_tests().await;

    let mut is_ok = true;

    for result in results {
        let TestResult {
            name: function_name,
            result,
        } = result;

        let results = match result {
            Ok(x) => x,
            Err(err) => {
                println!("[FAIL] {function_name}: {err:?}");
                is_ok = false;
                continue;
            }
        };

        for result in results {
            let SingleTestResult { name, result } = result;

            let result = match result {
                Ok(x) => x,
                Err(err) => {
                    println!("[FAIL] {function_name} > {name}: {:?}", err);
                    is_ok = false;
                    continue;
                }
            };

            if let Some(err) = result {
                println!("[FAIL] {function_name} > {name}:\n{:#?}", err);
                is_ok = false;
            } else {
                println!("[ OK ] {function_name} > {name}");
            }
        }
    }

    Ok(is_ok)
}
