//! blues-protocol build script.
//!
//! Compiles `proto/blues.proto` via tonic-build, using a vendored protoc
//! binary so contributors don't need a system protoc install.

fn main() {
    let proto = std::path::Path::new("proto/blues.proto");
    if !proto.exists() {
        println!("cargo:warning=blues-protocol: proto/blues.proto missing, skipping codegen");
        return;
    }

    let protoc = protoc_bin_vendored::protoc_bin_path()
        .expect("protoc-bin-vendored: no bundled protoc for this target");
    std::env::set_var("PROTOC", &protoc);

    println!("cargo:rerun-if-changed=proto/blues.proto");
    println!("cargo:rerun-if-changed=build.rs");

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&["proto/blues.proto"], &["proto"])
        .expect("compile blues.proto");
}
