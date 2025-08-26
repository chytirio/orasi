fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell cargo to rerun this if the proto files change
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.collector.trace.v1.proto");
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.collector.metrics.v1.proto");
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.collector.logs.v1.proto");
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.common.v1.proto");
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.resource.v1.proto");
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.trace.v1.proto");
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.metrics.v1.proto");
    println!("cargo:rerun-if-changed=proto/opentelemetry.proto.logs.v1.proto");

    // Compile the proto files
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute(
            ".",
            "#[cfg_attr(feature = \"with-serde\", derive(serde::Serialize, serde::Deserialize))]",
        )
        .type_attribute(
            ".",
            "#[cfg_attr(feature = \"with-serde\", serde(rename_all = \"camelCase\"))]",
        )
        .compile_protos(
            &[
                "proto/opentelemetry.proto.collector.trace.v1.proto",
                "proto/opentelemetry.proto.collector.metrics.v1.proto",
                "proto/opentelemetry.proto.collector.logs.v1.proto",
            ],
            &["proto"],
        )?;

    Ok(())
}
