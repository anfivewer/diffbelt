use std::rc::Rc;

use diffbelt_util::tokio_runtime::create_main_tokio_runtime;
use diffbelt_yaml::parse_yaml;

use crate::config_tests::run::run_tests;
use crate::config_tests::{SingleTestResult, TestResult};
use crate::CliConfig;

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

    let is_ok = run_tests(&config).await.expect("Running tests");

    assert!(is_ok);
}
