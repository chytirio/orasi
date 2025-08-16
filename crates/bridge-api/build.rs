fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun this if the proto files change
    println!("cargo:rerun-if-changed=proto/bridge.proto");
    println!("cargo:rerun-if-changed=proto/health.proto");
    
    // Compile the proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/bridge.proto", "proto/health.proto"], &["proto"])?;
    
    Ok(())
}
