fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = &[
        "proto/laurus/v1/common.proto",
        "proto/laurus/v1/index.proto",
        "proto/laurus/v1/document.proto",
        "proto/laurus/v1/search.proto",
        "proto/laurus/v1/health.proto",
    ];

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(proto_files, &["proto"])?;

    // Re-run if any proto file changes.
    for proto in proto_files {
        println!("cargo:rerun-if-changed={proto}");
    }

    Ok(())
}
