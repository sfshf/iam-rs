fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=aud/v1/aud.proto");
    println!("cargo:rerun-if-changed=authz/v1/authz.proto");
    println!("cargo:rerun-if-changed=domain/v1/domain.proto");
    println!("cargo:rerun-if-changed=identity/v1/identity.proto");
    println!("cargo:rerun-if-changed=event/v1/event.proto");
    println!("cargo:rerun-if-changed=daily/v1/daily.proto");

    // Optionally use protoc-gen-grpc-rust's protoc for prost. protoc-gen-grpc-rust will skip its
    // build when PROTOC_GEN_GRPC_RUST_NO_BUILD=1 (used in gRPC's CI), so we check that the binary
    // exists.
    #[cfg(feature = "protoc-gen-grpc-rust")]
    if protoc_gen_grpc_rust::protoc().exists() {
        unsafe {
            env::set_var("PROTOC", protoc_gen_grpc_rust::protoc());
        }
    }

    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(
            &[
                "aud/v1/aud.proto",
                "authz/v1/authz.proto",
                "domain/v1/domain.proto",
                "identity/v1/identity.proto",
                "event/v1/event.proto",
                "daily/v1/daily.proto",
            ],
            &[".", "vendor"],
        )?;

    Ok(())
}
