{
  pkgs,
  lib,
  stdenv,
  rustPlatform,
  rustc,
  cargo,
  makeDesktopItem,
}:
let
  pname = "defguard-client";
  version = "1.6.2"; # TODO: Get this from Cargo.toml or git

  desktopItem = makeDesktopItem {
    name = pname;
    exec = pname;
    icon = pname;
    desktopName = "Defguard";
    genericName = "Defguard VPN Client";
    categories = [
      "Network"
      "Security"
    ];
  };

  pnpm = pkgs.pnpm_10;

  buildInputs = with pkgs; [
    at-spi2-atk
    atkmm
    cairo
    dbus
    gdk-pixbuf
    glib
    glib-networking
    gtk3
    harfbuzz
    librsvg
    libsoup_3
    pango
    webkitgtk_4_1
    openssl
    libayatana-appindicator
    desktop-file-utils
    iproute2
  ];

  nativeBuildInputs = [
    rustc
    cargo
    pkgs.pkg-config
    pkgs.gobject-introspection
    pkgs.cargo-tauri
    pkgs.nodejs_24
    pkgs.protobuf
    pnpm
    # configures pnpm to use pre-fetched dependencies
    pnpm.configHook
    # configures cargo to use pre-fetched dependencies
    rustPlatform.cargoSetupHook
    # helper to add dynamic library paths
    pkgs.makeWrapper
    pkgs.wrapGAppsHook3
  ];
in
stdenv.mkDerivation (finalAttrs: rec {
  inherit
    pname
    version
    buildInputs
    nativeBuildInputs
    ;

  src = ../.;

  # prefetch cargo dependencies
  cargoRoot = "src-tauri";
  buildAndTestSubdir = "src-tauri";

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ../src-tauri/Cargo.lock;
  };

  # prefetch pnpm dependencies
  pnpmDeps = pnpm.fetchDeps {
    inherit (finalAttrs)
      pname
      version
      src
      ;

    fetcherVersion = 2;
    hash = "sha256-Xtn0FIq097sLEl/iodLeVVOYxVLx1ePJ8UjJUmgB2f0=";
  };

  buildPhase = ''
    pnpm tauri build
  '';

  installPhase = ''
    mkdir -p $out/bin

    # copy client binary
    cp src-tauri/target/release/${pname} $out/bin/

    # copy background service binary
    cp src-tauri/target/release/defguard-service $out/bin/

    # copy CLI binary
    cp src-tauri/target/release/dg $out/bin/

    # copy Tauri resources (icons for system tray, etc.)
    mkdir -p $out/lib/${pname}
    cp -r src-tauri/resources/* $out/lib/${pname}/

    mkdir -p $out/share/applications
    cp ${desktopItem}/share/applications/* $out/share/applications/
  '';

  # add extra args to wrapGAppsHook3 wrapper
  preFixup = ''
    gappsWrapperArgs+=(
      --prefix PATH : ${
        lib.makeBinPath [
          # `defguard-service` needs `ip` to manage wireguard
          pkgs.iproute2
          # `defguard-client` needs `update-desktop-database`
          pkgs.desktop-file-utils
        ]
      }
      --prefix LD_LIBRARY_PATH : ${
        lib.makeLibraryPath [
          pkgs.libayatana-appindicator
        ]
      }
    )
  '';

  meta = with lib; {
    description = "Defguard VPN Client";
    homepage = "https://defguard.net";
    # license = licenses.gpl3Only;
    maintainers = with maintainers; [ ];
    platforms = platforms.linux;
  };
})
