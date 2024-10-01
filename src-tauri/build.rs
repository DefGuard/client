use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // set VERGEN_GIT_SHA env variable based on git commit hash
    let git2 = Git2Builder::default().branch(true).sha(true).build()?;
    Emitter::default().add_instructions(&git2)?.emit()?;

    // compiling protos using path on build time
    let mut config = prost_build::Config::new();
    // enable optional fields
    config.protoc_arg("--experimental_allow_proto3_optional");
    // make sure empty DNS is deserialized correctly as None
    config.type_attribute(".DeviceConfig", "#[serde_as]");
    config.field_attribute(
        ".DeviceConfig.dns",
        "#[serde_as(deserialize_as = \"NoneAsEmptyString\")]",
    );
    // Make all messages serde-serializable
    config.type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]");
    tonic_build::configure().compile_protos_with_config(
        config,
        &["proto/client/client.proto", "proto/core/proxy.proto"],
        &["proto/client", "proto/core"],
    )?;

    tauri_build::build();

    Ok(())
}
