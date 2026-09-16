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
        // TODO: remove once it is used :)
        .type_attribute(
            ".defguard.client_types.MfaConfigAuthorizeResponse",
            "#[allow(unused)]",
        )
        .type_attribute(
            ".defguard.client_types.MfaConfigSendCodeResponse",
            "#[allow(unused)]",
        )
        .type_attribute(
            ".defguard.client_types.MfaConfigSendCodeRequest",
            "#[allow(unused)]",
        )
        .type_attribute(
            ".defguard.client_types.MfaConfigStartRequest",
            "#[allow(unused)]",
        )
        .compile_protos(&["../proto/v1/core/proxy.proto"], &["../proto"])?;

    Ok(())
}
