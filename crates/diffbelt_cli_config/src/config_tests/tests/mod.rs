use crate::config_tests::run::run_tests;
use crate::config_tests::RunTestsOptions;
use crate::wasm::engine::{WasmEngine, WasmEngineOptions};
use crate::CliConfig;
use diffbelt_http_client::client::{DiffbeltClient, DiffbeltClientNewOptions};
use diffbelt_util::tokio_runtime::create_main_tokio_runtime;
use diffbelt_yaml::parse_yaml;
use std::path::PathBuf;
use std::rc::Rc;

#[test]
fn run_example_config_tests() {
    let runtime = create_main_tokio_runtime().unwrap();
    runtime.block_on(run_example_config_tests_inner());
}

async fn run_example_config_tests_inner() {
    let config_str = include_str!("../../../../../examples/cli-config.yaml");

    let docs = parse_yaml(config_str).expect("parsing");
    let doc = &docs[0];
    let config = CliConfig::from_yaml(Rc::from("../../examples"), doc).expect("reading");

    let wasm_engine = WasmEngine::new(WasmEngineOptions {
        wasm_root_path: PathBuf::from(config.self_path.as_ref()),
    })
    .await
    .expect("engine creation");

    let options = RunTestsOptions {
        with_unit: true,
        with_integration: false,
        wasm_engine,
        client: None,
        cli_api: None,
    };

    let is_ok = run_tests(&config, options).await.expect("Running tests");

    assert!(is_ok);
}
