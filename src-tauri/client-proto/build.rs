fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../proto");

    tonic_prost_build::configure()
        // Enable optional fields.
        .protoc_arg("--experimental_allow_proto3_optional")
        // Make sure empty DNS is deserialized correctly as `None`.
        .type_attribute(".DeviceConfig", "#[serde_as]")
        .field_attribute(
            ".DeviceConfig.dns",
            "#[serde_as(deserialize_as = \"NoneAsEmptyString\")]",
        )
        // Make all messages serde-serializable.
        .type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]")
        .compile_protos(
            &[
                "../proto/v1/client/client.proto",
                "../proto/v1/core/proxy.proto",
                "../proto/enterprise/v2/posture/posture.proto",
                "../proto/common/client_types.proto",
            ],
            &["../proto"],
        )?;

    Ok(())
}
