use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            &["proto/client/client.proto", "proto/core/proxy.proto"],
            &["proto/client", "proto/core"],
        )?;

    tauri_build::build();

    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
