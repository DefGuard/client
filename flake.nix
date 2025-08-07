{
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";

    # include git submodules
    self.submodules = true;
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = ["rust-analyzer" "rust-src" "rustfmt" "clippy"];
        targets = ["wasm32-unknown-unknown" "x86_64-apple-darwin" "aarch64-apple-darwin" "x86_64-pc-windows-gnu"];
      };

      buildInputs = with pkgs; [
        gtk3
        cairo
        gdk-pixbuf
        glib
        dbus
        openssl
        librsvg
        libsoup_3
        webkitgtk_4_0
        libayatana-appindicator
        cargo-tauri_1
      ];

      nativeBuildInputs = with pkgs; [
        rustToolchain
        pkg-config
        cargo-tauri_1
        nodejs_24
        pnpm
        # configures pnpm to use pre-fetched dependencies
        pnpm.configHook
        protobuf
        # configures cargo to use pre-fetched dependencies
        rustPlatform.cargoSetupHook
        perl
      ];
    in {
      devShells.default = import ./nix/shell.nix {
        inherit pkgs buildInputs nativeBuildInputs rustToolchain;
      };

      packages.default = pkgs.callPackage ./nix/package.nix {
        inherit pkgs buildInputs nativeBuildInputs;
      };

      formatter = pkgs.alejandra;
    });
}
