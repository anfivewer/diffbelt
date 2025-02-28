use text_diff::Difference;
use thiserror::Error;

use diffbelt_yaml::YamlParsingError;

use crate::config_tests::error::AssertError;
use crate::config_tests::{RunTestsOptions, SingleTestResult, TestResult};
use crate::errors::ConfigParsingError;
use crate::CliConfig;

#[derive(Error, Debug)]
pub enum RunTestsError {
    #[error(transparent)]
    YamlParsing(YamlParsingError),
    #[error(transparent)]
    ConfigParsing(ConfigParsingError),
}

pub async fn run_tests(
    config: &CliConfig,
    options: RunTestsOptions<'_>,
) -> Result<bool, RunTestsError> {
    let results = config.run_tests(options).await;

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
                println!("[FAIL] {function_name} > {name}:");

                match err {
                    AssertError::ValueMissmatch { .. } => {
                        println!("{err:#?}");
                    }
                    AssertError::ExpectedErrorButSucceed { .. } => {
                        println!("{err:#?}");
                    }
                    AssertError::HasDiff { diffs } => {
                        fn print_diff_line(prefix: &str, s: &str) {
                            if s == "\n" {
                                println!("{prefix}");
                                return;
                            }

                            let lines = s.split("\n");
                            for line in lines {
                                println!("{prefix}{line}");
                            }
                        }

                        for difference in diffs {
                            match difference {
                                Difference::Same(s) => {
                                    print_diff_line("    ", &s);
                                }
                                Difference::Add(s) => {
                                    print_diff_line("  + ", &s);
                                }
                                Difference::Rem(s) => {
                                    print_diff_line("  - ", &s);
                                }
                            }
                        }
                    }
                }

                is_ok = false;
            } else {
                println!("[ OK ] {function_name} > {name}");
            }
        }
    }

    Ok(is_ok)
}
