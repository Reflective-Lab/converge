// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Build script for protobuf code generation.
//!
//! Compiles `context.proto` when the `grpc` feature is enabled and the proto file
//! exists. The shared `converge.v1` contract now comes from `converge-protocol`.

fn main() {
    #[cfg(feature = "grpc")]
    {
        let manifest_dir = std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo must set CARGO_MANIFEST_DIR for the converge-runtime build script"),
        );
        let schema_dir = manifest_dir.join("../../schema/proto");
        let context_proto = schema_dir.join("context.proto");
        if std::path::Path::new(&context_proto).exists() {
            println!("cargo:rerun-if-changed={}", context_proto.display());
            tonic_build::configure()
                .build_server(false)
                .build_client(true)
                .out_dir("src/ledger/generated")
                .compile_protos(&[&context_proto], &[schema_dir])
                .expect("Failed to compile context.proto");
        }
    }
}
