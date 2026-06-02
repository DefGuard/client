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

      defguard-client = pkgs.callPackage ./nix/package.nix {
        inherit pkgs craneLib;
      };
    in {
      devShells.default = import ./nix/shell.nix {
        pkgs = devPkgs;
        inherit crane;
      };

      packages = {
        default = defguard-client;
        inherit defguard-client;
        defguard-service =
          pkgs.runCommand "defguard-service" {
            nativeBuildInputs = [pkgs.makeWrapper];
          } ''
            mkdir -p $out/bin
            cp ${defguard-client}/bin/defguard-service $out/bin/
          '';
        dg =
          pkgs.runCommand "dg" {
            nativeBuildInputs = [pkgs.makeWrapper];
          } ''
            mkdir -p $out/bin
            cp ${defguard-client}/bin/dg $out/bin/
          '';
      };

      checks.default = defguard-client;

      formatter = pkgs.alejandra;
    })
    // {
      nixosModules.default = import ./nix/nixos-module.nix {mkCraneLib = crane.mkLib;};
    };
}
