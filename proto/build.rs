fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=aud/v1/aud.proto");
    println!("cargo:rerun-if-changed=authz/v1/authz.proto");
    println!("cargo:rerun-if-changed=domain/v1/domain.proto");
    println!("cargo:rerun-if-changed=identity/v1/identity.proto");
    println!("cargo:rerun-if-changed=event/v1/event.proto");

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
                // aud service's proto files
                "aud/v1/default.proto",
                "aud/v1/manage.proto",
                // authz service's proto files
                "authz/v1/default.proto",
                "authz/v1/manage.proto",
                // domain service's proto files
                "domain/v1/default.proto",
                "domain/v1/manage.proto",
                // identity service's proto files
                "identity/v1/default.proto",
                "identity/v1/user.proto",
                "identity/v1/auth.proto",
                "identity/v1/manage.proto",
                // event proto files
                "event/v1/default.proto",
            ],
            &[".", "vendor"],
        )?;

    Ok(())
}
