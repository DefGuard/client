{
  pkgs ? import <nixpkgs> {},
  buildInputs,
  nativeBuildInputs,
  rustToolchain,
}:
pkgs.mkShell {
  inherit buildInputs nativeBuildInputs;

  packages = with pkgs; [
    trunk
    sqlx-cli
    vtsls
  ];

  shellHook = with pkgs; ''
    export LD_LIBRARY_PATH="${
      lib.makeLibraryPath buildInputs
    }:$LD_LIBRARY_PATH"
    export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include/openssl"
    export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
    export OPENSSL_ROOT_DIR="${pkgs.openssl.out}"
    # https://discourse.nixos.org/t/which-package-includes-org-gtk-gtk4-settings-filechooser/38063/12
    export XDG_DATA_DIRS="${pkgs.gtk3}/share/gsettings-schemas/gtk+3-${pkgs.gtk3.dev.version}:$XDG_DATA_DIRS"
    export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
  '';
}
