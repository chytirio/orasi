fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun this if the proto files change
    println!("cargo:rerun-if-changed=proto/bridge.proto");
    println!("cargo:rerun-if-changed=proto/health.proto");

    // Compile the proto files
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)

        .type_attribute(
            ".",
            "#[cfg_attr(feature = \"with-schemars\", derive(schemars::JsonSchema))]",
        )
        .type_attribute(
            ".",
            "#[cfg_attr(feature = \"with-serde\", derive(serde::Serialize, serde::Deserialize))]",
        )
        .type_attribute(
            ".",
            "#[cfg_attr(feature = \"with-serde\", serde(rename_all = \"camelCase\"))]",
        )
        .compile_protos(&["proto/bridge.proto", "proto/health.proto"], &["proto"])?;

    Ok(())
}
