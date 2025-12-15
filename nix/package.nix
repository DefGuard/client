{
  pkgs,
  lib,
  stdenv,
  rustPlatform,
  rustc,
  cargo,
  makeDesktopItem,
}: let
  pname = "defguard-client";
  version = "1.6.1"; # TODO: Get this from Cargo.toml or git

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
    gtk4
    harfbuzz
    librsvg
    libsoup_3
    pango
    webkitgtk_4_1
    openssl
    libayatana-appindicator
    desktop-file-utils
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
    pnpmDeps = pkgs.pnpm.fetchDeps {
      inherit
        (finalAttrs)
        pname
        version
        src
        ;

      fetcherVersion = 2;
      hash = "sha256-B71cUUBmTI2fQpKsP3Ejk1lpyvVdaniqs1UFP8+Jg9g=";
    };

    buildPhase = ''
      pnpm tauri build
    '';

    postInstall = ''
      # copy client binary
      mkdir -p $out/bin
      cp src-tauri/target/release/${pname} $out/bin/
      # copy service binary
      mkdir -p $out/bin
      cp src-tauri/target/release/defguard-service $out/bin/
      # copy cli binary
      mkdir -p $out/bin
      cp src-tauri/target/release/dg $out/bin/

      # add required library to client binary RPATH
      wrapProgram $out/bin/${pname} \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [pkgs.libayatana-appindicator pkgs.desktop-file-utils]}

      mkdir -p $out/share/applications
      cp ${desktopItem}/share/applications/* $out/share/applications/
    '';

    meta = with lib; {
      description = "Defguard VPN Client";
      homepage = "https://defguard.net";
      # license = licenses.gpl3Only;
      maintainers = with maintainers; [];
      platforms = platforms.linux;
    };
  })
