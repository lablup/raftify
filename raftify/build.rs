fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .extern_path(".eraftpb", "::raft::eraftpb")
        .compile(&["proto/raft_service.proto"], &["proto/"])?;

    built::write_built_file().expect("Failed to acquire build-time information");
    Ok(())
}
