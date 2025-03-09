use crate::config_tests::error::{AssertError, TestError};
use crate::config_tests::options::RunTestsContext;
use crate::config_tests::{RunTestsOptions, SingleTestResult, TestResult};
use crate::util::init_collections::{
    init_collections, InitCollectionsError, InitCollectionsOptions,
};
use crate::wasm::integration_tests::WasmIntegrationTestFunctions;
use crate::wasm::{NewWasmInstanceOptions, WasmModuleInstance};
use crate::CliConfig;
use diffbelt_util::diffbelt::stdout::{DiffbeltStdout, DiffbeltStdoutOptions};
use diffbelt_util::fs::temp_dir::TempDir;
use serde::Deserialize;
use std::process::Stdio;
use std::rc::Rc;
use std::time::Instant;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
pub struct IntegrationTestDef {
    pub name: Rc<str>,
    pub wasm: Rc<str>,
}

impl CliConfig {
    pub async fn run_integration_tests(
        &self,
        results: &mut Vec<TestResult>,
        options: &RunTestsOptions,
        context: &mut RunTestsContext,
    ) {
        for test in &self.integration_tests {
            let start = Instant::now();

            match self.run_integration_test(test, options, context).await {
                Ok(assert_error) => results.push(TestResult {
                    name: Rc::from("Integration"),
                    result: Ok(vec![SingleTestResult {
                        name: test.name.clone(),
                        result: Ok(assert_error),
                        elapsed: Some(start.elapsed()),
                    }]),
                }),
                Err(err) => results.push(TestResult {
                    name: Rc::from("Integration"),
                    result: Ok(vec![SingleTestResult {
                        name: test.name.clone(),
                        result: Err(err),
                        elapsed: Some(start.elapsed()),
                    }]),
                }),
            }
        }
    }

    pub async fn run_integration_test(
        &self,
        test: &IntegrationTestDef,
        options: &RunTestsOptions,
        context: &mut RunTestsContext,
    ) -> Result<Option<AssertError>, TestError> {
        let client = options
            .client
            .as_ref()
            .ok_or_else(|| TestError::Unspecified(String::from("missing client")))?
            .as_ref();

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

        let wasm_mod = context.wasm_engine.get_module(module_name).await?;

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

        let mut stdout = DiffbeltStdout::new(DiffbeltStdoutOptions { read: stdout });

        stdout.wait_for_idle().await?;

        let () = init_collections(InitCollectionsOptions {
            client,
            collections: &self.collections,
            print_before_create: |_| {},
            print_after_create: |_| {},
        })
        .await
        .map_err(|err| match err {
            InitCollectionsError::Message(msg) => TestError::Unspecified(msg),
            InitCollectionsError::DiffbeltClient(err) => err.into(),
        })?;

        stdout.wait_for_idle().await?;

        let wasm_instance = WasmModuleInstance::new(NewWasmInstanceOptions {
            engine: &mut context.wasm_engine,
            module: &wasm_mod,
            requests: context.requests.clone(),
        })
        .await?;

        let test_functions = WasmIntegrationTestFunctions::new(&wasm_instance, function_name)?;

        let test_result_error = test_functions.call_test().await?;

        if let Some(error) = test_result_error {
            return Ok(Some(AssertError::Message(error)));
        }

        Ok(None)
    }
}
