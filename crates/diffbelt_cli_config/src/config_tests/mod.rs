use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_yaml::YamlNodeRc;
use error::{AssertError, TestError};
use serde::Deserialize;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config_tests::options::RunTestsContext;
use crate::config_tests::transforms::{TransformTest, TransformTestCreator};
use crate::requests::client_impl::DiffbeltRequestsClientImpl;
use crate::wasm::engine::{WasmEngine, WasmEngineOptions};
use crate::wasm::WasmError;
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
    pub elapsed: Option<Duration>,
}

#[derive(Debug)]
pub struct TestResult {
    pub name: Rc<str>,
    pub result: Result<Vec<SingleTestResult>, TestError>,
}

impl CliConfig {
    pub async fn run_tests(&self, options: RunTestsOptions) -> Vec<TestResult> {
        let mut result = Vec::new();

        let context = 'outer: {
            let wasm_root_path = PathBuf::from(self.self_path.as_ref());
            let wasm_engine = match WasmEngine::new(WasmEngineOptions { wasm_root_path }).await {
                Ok(x) => x,
                Err(err) => {
                    break 'outer Err(err);
                }
            };
            let client = match options.client.as_ref() {
                Some(client) => client.clone(),
                None => {
                    break 'outer Err(WasmError::Unspecified(String::from("No DiffbeltClient")));
                }
            };
            let (requests, requests_impl) = DiffbeltRequestsClientImpl::new(client);
            let mut context = RunTestsContext {
                wasm_engine,
                requests: Arc::new(requests),
                requests_impl,
            };

            if let Err(err) = self.init_run_tests_context(&mut context).await {
                break 'outer Err(err);
            }

            Ok(context)
        };

        let mut context = match context {
            Ok(x) => x,
            Err(err) => {
                result.push(TestResult {
                    name: Rc::from("integration"),
                    result: Err(TestError::Wasm(err)),
                });
                return result;
            }
        };

        if options.with_unit {
            self.run_unit_tests(&mut result, &mut context).await;
        }

        if options.with_integration {
            self.run_integration_tests(&mut result, &options, &mut context)
                .await;
        }

        result
    }
}
