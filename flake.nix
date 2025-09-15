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
