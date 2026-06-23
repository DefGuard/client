use std::env;

// On macOS, the binary must include `Info.plist` to be properly signed and be allowed to run.
fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "macos" {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let plist = format!("{manifest_dir}/Info.plist");
        println!("cargo:rerun-if-changed={plist}");
        println!("cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{plist}");
    }
}
