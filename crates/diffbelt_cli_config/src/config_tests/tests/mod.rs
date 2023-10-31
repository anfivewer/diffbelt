use std::rc::Rc;

use diffbelt_util::tokio_runtime::create_main_tokio_runtime;
use diffbelt_yaml::parse_yaml;

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
                println!("[FAIL] {function_name}: {:?}", err);
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
                println!("[FAIL] {function_name} > {name}: {:?}", err);
                is_ok = false;
            } else {
                println!("[ OK ] {function_name} > {name}");
            }
        }
    }

    assert!(is_ok);
}
