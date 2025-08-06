{
  pkgs,
  lib,
  stdenv,
  rustPlatform,
  makeDesktopItem,
  buildInputs,
  nativeBuildInputs,
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
in
  stdenv.mkDerivation (finalAttrs: rec {
    inherit pname version buildInputs nativeBuildInputs;

    src = ../.;

    cargoRoot = "src-tauri";
    buildAndTestSubdir = "src-tauri";

    cargoDeps = rustPlatform.importCargoLock {
      lockFile = ../src-tauri/Cargo.lock;
      outputHashes = {
        "defguard_wireguard_rs-0.7.4" = "sha256-pxwN43BntOEYtp+TlpQFX78gg1ko4zuXEGctZIfSrhg=";
        "tauri-plugin-log-0.0.0" = "sha256-jGzlN/T29Hya4bKe9Dwl2mRRFLXMywrHk+32zgwrpJ0=";
      };
    };

    pnpmDeps = pkgs.pnpm.fetchDeps {
      inherit
        (finalAttrs)
        pname
        version
        ;

      src = ../.;

      fetcherVersion = 2;
      hash = "sha256-OIm8OCvE7V77uuWsfeY/ax7T5SSE9Rt0EKrsnuDzOYk=";
    };

    configurePhase = ''
      export HOME=$TMPDIR
      pnpm config set store-dir ${pnpmDeps}
      pnpm config set offline true
      pnpm install --frozen-lockfile --ignore-scripts
    '';

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
