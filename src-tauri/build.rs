use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=DEFGUARD_CLIENT_BUILD_VERSION");

    println!("cargo:rerun-if-env-changed=DEFGUARD_CLIENT_DEV");
    println!("cargo::rustc-check-cfg=cfg(defguard_client_dev)");
    if std::env::var("DEFGUARD_CLIENT_DEV").is_ok() {
        println!("cargo::rustc-cfg=defguard_client_dev");
    }

    // set VERGEN_GIT_SHA env variable based on git commit hash
    let git2 = Git2Builder::default().branch(true).sha(true).build()?;
    Emitter::default().add_instructions(&git2)?.emit()?;

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
                "proto/v1/client/client.proto",
                "proto/v1/core/proxy.proto",
                "proto/enterprise/v2/posture/posture.proto",
                "proto/common/client_types.proto",
            ],
            &["proto"],
        )?;

    tauri_build::build();

    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
