fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .compile_protos(&["../proto/cert_agent.proto"], &["../proto"])?;
    Ok(())
}
