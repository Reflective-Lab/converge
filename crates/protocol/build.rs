// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let bundled_proto_dir = manifest_dir.join("proto");
    let workspace_proto_dir = manifest_dir.join("../../schema/proto");

    let proto_dir = if bundled_proto_dir.join("converge.proto").exists() {
        bundled_proto_dir
    } else {
        workspace_proto_dir
    };
    let proto_file = proto_dir.join("converge.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile_protos(&[proto_file], &[proto_dir])?;

    Ok(())
}
