use std::path::PathBuf;
use std::process::Command;

enum SubPath {
    File(&'static str),
    #[allow(dead_code)]
    Folder((&'static str, Vec<SubPath>)),
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/protos/log_line.fbs");

    let paths = [SubPath::File("log_line.fbs")];

    fn process_path(prefix: PathBuf, path: SubPath) {
        match path {
            SubPath::File(file) => {
                let mut generated_path = PathBuf::from("src/protos/generated");
                generated_path.push(&prefix);

                let mut fbs_path = PathBuf::from("src/protos");
                fbs_path.push(&prefix);
                fbs_path.push(file);

                let status = Command::new("flatc")
                    .args(&[
                        "--rust",
                        "-o",
                        generated_path.to_str().unwrap(),
                        fbs_path.to_str().unwrap(),
                    ])
                    .status()
                    .unwrap();

                assert!(status.success());
            }
            SubPath::Folder((path, parts)) => {
                for part in parts {
                    let mut prefix = prefix.clone();
                    prefix.push(path);

                    process_path(prefix, part);
                }
            }
        }
    }

    for sub_path in paths {
        process_path(PathBuf::from("."), sub_path);
    }
}