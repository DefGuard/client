fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../proto");

    tonic_prost_build::configure()
        // These types contain sensitive data.
        .skip_debug(["SaveServiceLocationsRequest"])
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
        // `ServiceLocation` is persisted as JSON by the daemon. Tolerate these fields being absent
        // in files written by older clients. Deliberately per-field rather than a container-level
        // `#[serde(default)]`, so a truncated or corrupt file still fails to deserialize instead of
        // quietly becoming "no locations".
        .field_attribute(
            ".defguard.client.v1.ServiceLocation.core_location_id",
            "#[serde(default)]",
        )
        .field_attribute(
            ".defguard.client.v1.ServiceLocation.posture_check_required",
            "#[serde(default)]",
        )
        // [2.2] These repeated fields are absent in responses from pre-2.2 edges.
        .field_attribute(
            ".defguard.client_types.DeviceConfig.steps",
            "#[serde(default)]",
        )
        .field_attribute(
            ".defguard.client_types.ClientMfaStartResponse.rejections",
            "#[serde(default)]",
        )
        .field_attribute(
            ".defguard.client_types.ClientMfaStartResponse.credential_ids",
            "#[serde(default)]",
        )
        .field_attribute(
            ".defguard.client_types.ClientMfaStepStartResponse.credential_ids",
            "#[serde(default)]",
        )
        // Use proto defaults for missing fields in enrollment types that
        // may differ across proxy versions.
        .type_attribute(".defguard.client_types.AdminInfo", "#[serde(default)]")
        .type_attribute(
            ".defguard.client_types.InitialUserInfo",
            "#[serde(default)]",
        )
        .type_attribute(
            ".defguard.client_types.EnrollmentSettings",
            "#[serde(default)]",
        )
        .type_attribute(".defguard.client_types.InstanceInfo", "#[serde(default)]")
        .type_attribute(
            ".defguard.client_types.EnrollmentStartResponse",
            "#[serde(default)]",
        )
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
