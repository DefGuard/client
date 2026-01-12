{
  pkgs,
  lib,
  stdenv,
  rustPlatform,
  rustc,
  cargo,
  makeDesktopItem,
  pnpmConfigHook,
  fetchPnpmDeps,
}: let
  pname = "defguard-client";
  # Automatically read version from Cargo.toml
  version = (fromTOML (builtins.readFile ../src-tauri/Cargo.toml)).workspace.package.version;

  desktopItem = makeDesktopItem {
    name = pname;
    exec = pname;
    icon = pname;
    desktopName = "Defguard";
    genericName = "Defguard VPN Client";
    categories = ["Network" "Security"];
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
    libayatana-indicator
    ayatana-ido
    libdbusmenu-gtk3
    desktop-file-utils
    iproute2
    lsb-release
    openresolv
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
    pnpmConfigHook
    # configures cargo to use pre-fetched dependencies
    rustPlatform.cargoSetupHook
    # helper to add runtime binary & library deps paths
    pkgs.makeWrapper
    pkgs.wrapGAppsHook3
  ];
in
  stdenv.mkDerivation (finalAttrs: rec {
    inherit pname version buildInputs nativeBuildInputs;

    src = ../.;

    # prefetch cargo dependencies
    cargoRoot = "src-tauri";
    buildAndTestSubdir = "src-tauri";

    cargoDeps = rustPlatform.importCargoLock {
      lockFile = ../src-tauri/Cargo.lock;
    };

    # prefetch pnpm dependencies
    pnpmDeps = fetchPnpmDeps {
      inherit
        (finalAttrs)
        pname
        version
        src
        ;

      fetcherVersion = 2;
      hash = "sha256-co5UK8FHDIO7bDRS4yxczz7VxGNWlPBd8h0PlmOQ3xM=";
    };

    buildPhase = ''
      runHook preBuild

      pnpm tauri build --verbose

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin

      # copy client binary
      install -Dm755 src-tauri/target/release/${pname} $out/bin/${pname}

      # copy background service binary
      install -Dm755 src-tauri/target/release/defguard-service $out/bin/defguard-service

      # copy CLI binary
      install -Dm755 src-tauri/target/release/dg $out/bin/dg

      # Copy resources directory (for tray icons, etc.)
      mkdir -p $out/lib/${pname}
      cp -r src-tauri/resources $out/lib/${pname}/

      # install desktop entry
      mkdir -p $out/share/applications
      cp ${desktopItem}/share/applications/* $out/share/applications/

      # install icon files
      mkdir -p $out/share/icons/hicolor/{32x32,128x128}/apps
      install -Dm644 src-tauri/icons/32x32.png $out/share/icons/hicolor/32x32/apps/${pname}.png
      install -Dm644 src-tauri/icons/128x128.png $out/share/icons/hicolor/128x128/apps/${pname}.png

      runHook postInstall
    '';

    # add extra args to wrapGAppsHook3 wrapper
    preFixup = ''
      gappsWrapperArgs+=(
        --prefix PATH : ${
        lib.makeBinPath [
          # `defguard-service` needs `ip` to manage WireGuard
          pkgs.iproute2
          # `defguard-service` needs `resolvconf` to manage DNS
          pkgs.openresolv
          # `defguard-client` needs `update-desktop-database` and `lsb_release`
          pkgs.desktop-file-utils
          pkgs.lsb-release
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
      maintainers = with maintainers; [wojcik91];
      platforms = platforms.linux;
    };
  })
