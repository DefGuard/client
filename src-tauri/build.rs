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

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "macos"
        && std::env::var("CARGO_FEATURE_MACOS_INSTALLER").is_ok()
    {
        println!("cargo:rustc-link-lib=framework=SystemExtensions");
    }

    tauri_build::build();
    Ok(())
}
