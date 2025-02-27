use crate::config_tests::error::TestError;
use crate::config_tests::{SingleTestResult, TestResult};
use crate::CliConfig;
use diffbelt_util::fs::temp_dir::TempDir;
use serde::Deserialize;
use std::process::Stdio;
use std::rc::Rc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Deserialize)]
pub struct IntegrationTestDef {
    pub name: Rc<str>,
    pub wasm: Rc<str>,
}

impl CliConfig {
    pub async fn run_integration_tests(&self, results: &mut Vec<TestResult>) {
        for test in &self.integration_tests {
            match run_integration_test(test).await {
                Ok(()) => results.push(TestResult {
                    name: test.name.clone(),
                    result: Ok(vec![SingleTestResult {
                        name: test.name.clone(),
                        result: Ok(None),
                    }]),
                }),
                Err(err) => results.push(TestResult {
                    name: test.name.clone(),
                    result: Ok(vec![SingleTestResult {
                        name: test.name.clone(),
                        result: Err(err),
                    }]),
                }),
            }
        }
    }
}

async fn run_integration_test(test: &IntegrationTestDef) -> Result<(), TestError> {
    let (module_name, function_name) = {
        let mut s = test.wasm.split('.');
        let module_name = s
            .next()
            .ok_or_else(|| TestError::Unspecified(String::from("Invalid wasm field format")))?;
        let function_name = s
            .next()
            .ok_or_else(|| TestError::Unspecified(String::from("Invalid wasm field format")))?;
        if let Some(_) = s.next() {
            return Err(TestError::Unspecified(String::from(
                "Invalid wasm field format: too much dots",
            )));
        }

        (module_name, function_name)
    };

    let temp_dir = TempDir::new()?;

    let mut command = Command::new("diffbelt");
    let command = command
        .kill_on_drop(true)
        .env("DIFFBELT_DATA_PATH", temp_dir.get_path_buf())
        .stdout(Stdio::piped());

    let mut child = command.spawn()?;

    let Some(stdout) = child.stdout.take() else {
        return Err(TestError::Unspecified(String::from("No stdout")));
    };

    let buf_reader = BufReader::new(stdout);
    let mut stdout_lines = buf_reader.lines();

    // TODO: parse port number and tell server to start on 0 port (any available)
    // Wait for startup
    while let Some(line) = stdout_lines.next_line().await? {
        if &line == "IDLE" {
            break;
        }
    }

    //

    Ok(())
}
