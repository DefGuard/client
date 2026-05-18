{
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";

    # let git manage submodules
    self.submodules = true;
    proto = {
      url = "path:src-tauri/proto";
      flake = false;
    };
    defguard-ui = {
      url = "path:src/shared/defguard-ui";
      flake = false;
    };
    boringtun = {
      url = "path:swift/boringtun";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      # Plain nixpkgs — used for packages and checks.
      pkgs = import nixpkgs {inherit system;};

      # nixpkgs with rust-overlay — only needed for the dev shell, which uses
      # pkgs.rust-bin to get a customised Rust toolchain.
      devPkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };

      craneLib = crane.mkLib pkgs;
    in {
      devShells.default = import ./nix/shell.nix {pkgs = devPkgs;};

      packages.default = pkgs.callPackage ./nix/package.nix {
        inherit pkgs craneLib;
      };

      checks.default = self.packages.${system}.default;

      formatter = pkgs.alejandra;
    })
    // {
      nixosModules.default = import ./nix/nixos-module.nix {mkCraneLib = crane.mkLib;};
    };
}
