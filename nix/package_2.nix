{
  lib,
  stdenv,
  rustPlatform,
  cargo-tauri_1,
  pkg-config,
  dbus,
  openssl,
  glib,
  gtk3,
  libsoup_2_4,
  webkitgtk_4_0,
  librsvg,
  protobuf,
  libayatana-appindicator,
  nodejs_20,
  nodePackages,
}:
rustPlatform.buildRustPackage {
  pname = "defguard-client";
  version = "0.1.0";

  src = ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    cargo-tauri_1
    cargo-tauri_1.hook
    pkg-config
    protobuf
    nodejs_20
    nodePackages.pnpm
  ];

  buildInputs = [
    dbus
    openssl
    glib
    gtk3
    libsoup_2_4
    webkitgtk_4_0
    librsvg
    libayatana-appindicator
  ];

  # Specify frontend distribution directory
  tauriFrontendDist = "dist";

  # Configure the frontend build
  preBuild = ''
    export HOME=$(mktemp -d)
    pnpm config set store-dir $HOME/.pnpm-store
    pnpm install --frozen-lockfile
  '';

  meta = with lib; {
    description = "Defguard desktop client";
    homepage = "https://github.com/defguard/client";
    license = licenses.asl20;
    maintainers = [];
    platforms = platforms.linux;
  };
}
