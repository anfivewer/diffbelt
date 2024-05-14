fn main() {
    println!("cargo:rerun-if-changed=src/protos/database_meta.proto");

    protobuf_codegen::Codegen::new()
        .out_dir("src/protos")
        .include("src/protos")
        .input("src/protos/database_meta.proto")
        .run()
        .expect("Protobuf codegen fail");
}
