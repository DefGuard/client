{
  pkgs,
  crane,
}: let
  # add development-related cargo tooling
  rustToolchain = pkgs.rust-bin.stable.latest.default.override {
    extensions = ["rust-analyzer" "rust-src" "rustfmt" "clippy"];
    targets = ["x86_64-apple-darwin" "aarch64-apple-darwin" "x86_64-pc-windows-gnu"];
  };

  # nightly rustfmt, needed only for the unstable import-grouping options that
  # `fmt-imports` passes via --config. It is deliberately NOT placed on PATH
  # (that would collide with the stable rustfmt above); the wrapper points
  # stable `cargo fmt` at it via RUSTFMT, leaving the default toolchain alone.
  # The unstable options live only here (not in a committed rustfmt.toml), so a
  # normal `cargo fmt` sees no unstable keys and stays warning-free.
  rustfmtNightly = pkgs.rust-bin.nightly.latest.rustfmt;

  # Usage: fmt-imports [cargo fmt flags]   e.g. fmt-imports --check
  fmtImports = pkgs.writeShellScriptBin "fmt-imports" ''
    set -euo pipefail
    root="$(${pkgs.git}/bin/git rev-parse --show-toplevel)"
    cd "$root/src-tauri"
    export RUSTFMT="${rustfmtNightly}/bin/rustfmt"
    exec ${rustToolchain}/bin/cargo fmt "$@" -- \
      --config imports_granularity=Crate,group_imports=StdExternalCrate
  '';

  craneLib = crane.mkLib pkgs;

  defguard-client = pkgs.callPackage ./package.nix {
    inherit craneLib;
    cargo = rustToolchain;
    rustc = rustToolchain;
  };

  # runtime libraries needed to run the dev server
  libraries = with pkgs; [
    libayatana-appindicator
  ];
in
  pkgs.mkShell {
    # inherit build inputs from the package
    inputsFrom = [defguard-client];

    # add additional dev tools
    packages = with pkgs; [
      rustToolchain
      fmtImports
      trunk
      sqlx-cli
      cargo-nextest
      vtsls
      trivy
    ];

    shellHook = with pkgs; ''
      export LD_LIBRARY_PATH="${
        lib.makeLibraryPath libraries
      }:$LD_LIBRARY_PATH"
      export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include/openssl"
      export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
      export OPENSSL_ROOT_DIR="${pkgs.openssl.out}"
      # https://discourse.nixos.org/t/which-package-includes-org-gtk-gtk4-settings-filechooser/38063/12
      export XDG_DATA_DIRS="${pkgs.gtk3}/share/gsettings-schemas/gtk+3-${pkgs.gtk3.dev.version}:$XDG_DATA_DIRS"
      export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
    '';
  }
