fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        // Enable a protoc experimental feature.
        .protoc_arg("--experimental_allow_proto3_optional")
        // Serialize empty DNS as None.
        .type_attribute(".DeviceConfig", "#[serde_as]")
        .field_attribute(
            ".DeviceConfig.dns",
            "#[serde_as(deserialize_as = \"NoneAsEmptyString\")]",
        )
        // Make all messages serde-serializable.
        .type_attribute(".", "#[derive(serde::Deserialize,serde::Serialize)]")
        .compile_protos(&["../proto/core/proxy.proto"], &["../proto/core"])?;

    Ok(())
}
