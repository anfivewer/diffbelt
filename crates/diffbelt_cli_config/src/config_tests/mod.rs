use std::ops::Deref;
use std::rc::Rc;

use serde::Deserialize;

use diffbelt_yaml::YamlNodeRc;
use error::{AssertError, TestError};

use crate::config_tests::transforms::{TransformTest, TransformTestCreator};
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
    pub async fn run_tests(&self, options: RunTestsOptions) -> Vec<TestResult> {
        let mut result = Vec::new();

        if options.with_unit {
            self.run_unit_tests(&mut result).await;
        }

        if options.with_integration {
            self.run_integration_tests(&mut result).await;
        }

        result
    }
}
