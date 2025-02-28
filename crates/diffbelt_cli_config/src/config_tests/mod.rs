use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;

use serde::Deserialize;

use diffbelt_yaml::YamlNodeRc;
use error::{AssertError, TestError};

use crate::config_tests::options::RunTestsContext;
use crate::config_tests::transforms::{TransformTest, TransformTestCreator};
use crate::wasm::engine::{WasmEngine, WasmEngineOptions};
use crate::CliConfig;
pub use options::RunTestsOptions;

pub mod compare;
pub mod error;
pub mod integration;
mod options;
pub mod run;
#[cfg(test)]
mod tests;
pub mod transforms;
mod unit;
pub mod value;
pub mod wasm;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct TestSuite {
    pub tests: Vec<SingleTest>,
}

#[derive(Debug, Deserialize)]
pub struct SingleTest {
    pub name: Rc<str>,
    pub input: YamlNodeRc,
    pub output: YamlNodeRc,
}

#[derive(Debug)]
pub struct SingleTestResult {
    pub name: Rc<str>,
    pub result: Result<Option<AssertError>, TestError>,
}

#[derive(Debug)]
pub struct TestResult {
    pub name: Rc<str>,
    pub result: Result<Vec<SingleTestResult>, TestError>,
}

impl CliConfig {
    pub async fn run_tests(&self, options: RunTestsOptions<'_>) -> Vec<TestResult> {
        let mut result = Vec::new();

        if options.with_unit {
            self.run_unit_tests(&mut result).await;
        }

        let integration_result = 'outer: {
            if options.with_integration {
                let wasm_root_path = PathBuf::from(self.self_path.as_ref());
                let wasm_engine = match WasmEngine::new(WasmEngineOptions { wasm_root_path }).await
                {
                    Ok(x) => x,
                    Err(err) => {
                        break 'outer Err(err);
                    }
                };
                let mut context = RunTestsContext { wasm_engine };

                if let Err(err) = self.init_run_tests_context(&mut context).await {
                    break 'outer Err(err);
                }

                self.run_integration_tests(&mut result, &options, &mut context)
                    .await;
            }

            Ok(())
        };

        if let Err(err) = integration_result {
            result.push(TestResult {
                name: Rc::from("integration"),
                result: Err(TestError::Wasm(err)),
            });
        }

        result
    }
}
