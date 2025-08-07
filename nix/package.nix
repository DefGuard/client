{
  pkgs,
  lib,
  stdenv,
  rustPlatform,
  makeDesktopItem,
}: let
  pname = "defguard-client";
  version = "1.5.0"; # TODO: Get this from Cargo.toml or git

  desktopItem = makeDesktopItem {
    name = pname;
    exec = pname;
    icon = pname;
    desktopName = "Defguard";
    genericName = "Defguard VPN Client";
    categories = ["Network" "Security"];
  };

  rustToolchain = pkgs.rust-bin.stable.latest.default;

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
in
  stdenv.mkDerivation (finalAttrs: rec {
    inherit pname version buildInputs nativeBuildInputs;

    src = ../.;

    # prefetch cargo dependencies
    cargoRoot = "src-tauri";
    buildAndTestSubdir = "src-tauri";

    cargoDeps = rustPlatform.importCargoLock {
      lockFile = ../src-tauri/Cargo.lock;
      # specify hashes for git dependencies
      outputHashes = {
        "defguard_wireguard_rs-0.7.4" = "sha256-pxwN43BntOEYtp+TlpQFX78gg1ko4zuXEGctZIfSrhg=";
        "tauri-plugin-log-0.0.0" = "sha256-jGzlN/T29Hya4bKe9Dwl2mRRFLXMywrHk+32zgwrpJ0=";
      };
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
      hash = "sha256-lQUhwy1/zz3mUN7wNLQyKeULYPQiu2UvMS2REu9XxEc=";
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
