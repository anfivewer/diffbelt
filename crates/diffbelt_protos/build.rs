use std::path::PathBuf;
use std::process::Command;

#[derive(Clone)]
struct FileOptions {
    include_prefix: Option<&'static str>,
}

enum SubPath {
    File((&'static str, Option<FileOptions>)),
    Folder((&'static str, Vec<SubPath>, Option<FileOptions>)),
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let paths = [
        SubPath::Folder((
            "transform",
            vec![
                SubPath::File(("map_filter.fbs", None)),
                SubPath::File(("aggregate.fbs", None)),
            ],
            None,
        )),
        SubPath::Folder((
            "api",
            vec![
                SubPath::File((
                    "collection.fbs",
                    None,
                )),
                SubPath::File((
                    "common.fbs",
                    None,
                )),
                SubPath::File((
                    "generation.fbs",
                    None,
                )),
                SubPath::File((
                    "get_keys_around.fbs",
                    None,
                )),
                SubPath::File((
                    "methods.fbs",
                    None,
                )),
                SubPath::File((
                    "phantom.fbs",
                    None,
                )),
                SubPath::File((
                    "put_many.fbs",
                    None,
                )),
                SubPath::File((
                    "query.fbs",
                    None,
                )),
            ],
            Some(FileOptions {
                include_prefix: Some("protos::generated::api"),
            })
        )),
    ];

    fn process_path(prefix: PathBuf, path: SubPath, dir_opts: Option<FileOptions>) {
        match path {
            SubPath::File((file, opts)) => {
                let opts = opts.or(dir_opts);

                let mut generated_path = PathBuf::from("src/protos/generated");
                generated_path.push(&prefix);

                let mut fbs_path = PathBuf::from("src/protos");
                fbs_path.push(&prefix);
                fbs_path.push(file);

                let fbs_path = fbs_path.to_str().unwrap();

                println!("cargo:rerun-if-changed={fbs_path}");

                let generated_path = generated_path.to_str().expect("invalid path");

                let mut args = Vec::with_capacity(6);
                args.push("--rust");

                if let Some(opts) = opts {
                    if let Some(include_prefix) = opts.include_prefix {
                        args.push("--include-prefix");
                        args.push(include_prefix);
                    }
                }

                args.push("-o");
                args.push(generated_path);
                args.push(fbs_path);

                let status = Command::new("flatc").args(&args).status().unwrap();

                assert!(status.success());
            }
            SubPath::Folder((path, parts, opts)) => {
                for part in parts {
                    let mut prefix = prefix.clone();
                    prefix.push(path);

                    process_path(prefix, part, opts.clone());
                }
            }
        }
    }

    for sub_path in paths {
        process_path(PathBuf::from("."), sub_path, None);
    }
}
