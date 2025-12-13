{
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";

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
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      # add rust overlay
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };
    in {
      devShells.default = import ./nix/shell.nix {
        inherit pkgs;
      };

      packages.default = pkgs.callPackage ./nix/package.nix {
        inherit pkgs;
      };

      formatter = pkgs.alejandra;
    })
    // {
      nixosModules.default = import ./nix/nixos-module.nix;
    };
}
