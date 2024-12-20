fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    // Enable a protoc experimental feature.
    config.protoc_arg("--experimental_allow_proto3_optional");
    // Serialize empty DNS as None.
    config.type_attribute(".DeviceConfig", "#[serde_as]");
    config.field_attribute(
        ".DeviceConfig.dns",
        "#[serde_as(deserialize_as = \"NoneAsEmptyString\")]",
    );
    // Make all messages serde-serializable.
    config.type_attribute(".", "#[derive(serde::Deserialize,serde::Serialize)]");
    config.compile_protos(&["../proto/core/proxy.proto"], &["../proto/core"])?;

    Ok(())
}
