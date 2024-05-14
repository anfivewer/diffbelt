use std::env;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::Command;

fn diffbelt_example_wasm_build_rs() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("No CARGO_MANIFEST_DIR var");

    let mut wasm_crate_path = PathBuf::from(manifest_dir);

    wasm_crate_path.push("../diffbelt_example_wasm");

    let mut wasm_src_path = wasm_crate_path.clone();
    wasm_src_path.push("src");
    let wasm_src_path = wasm_src_path.canonicalize().expect("Cannot canonicalize");

    let mut stack = Vec::new();

    stack.push(wasm_src_path);

    while let Some(path) = stack.pop() {
        let files = read_dir(path).expect("Cannot read wasm src dir");

        for entry in files {
            let entry = entry.expect("Cannot read wasm src dir");
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let path = path.to_str().expect("PathBuf is not a string");

            println!("cargo:rerun-if-changed={path}");
        }
    }

    let status = Command::new("make")
        .current_dir(&wasm_crate_path)
        .status()
        .expect("failed to run make");

    assert!(status.success(), "Wasm build is not success");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    diffbelt_example_wasm_build_rs();
}
