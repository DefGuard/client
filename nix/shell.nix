{pkgs ? import <nixpkgs> {}}: let
  # add development-related cargo tooling
  rustToolchain = pkgs.rust-bin.stable.latest.default.override {
    extensions = ["rust-analyzer" "rust-src" "rustfmt" "clippy"];
    targets = ["x86_64-apple-darwin" "aarch64-apple-darwin" "x86_64-pc-windows-gnu"];
  };

  rustPlatform = pkgs.makeRustPlatform {
    cargo = rustToolchain;
    rustc = rustToolchain;
  };

  defguard-client = pkgs.callPackage ./package.nix {inherit rustPlatform;};

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
      trunk
      sqlx-cli
      vtsls
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
