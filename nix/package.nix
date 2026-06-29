{
  pkgs,
  lib,
  craneLib,
  rustc,
  cargo,
  makeDesktopItem,
  pnpmConfigHook,
  fetchPnpmDeps,
}: let
  pname = "defguard-client";
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
  ];

  # Rust/cargo inputs shared by buildDepsOnly and the main build.
  cargoNativeBuildInputs = [
    rustc
    cargo
    pkgs.pkg-config
    pkgs.gobject-introspection
    pkgs.cargo-tauri
    pkgs.protobuf
  ];

  # Source filter for buildDepsOnly: Cargo files plus extras needed by build.rs
  # (proto files, tauri configs, capabilities, sqlx offline cache).
  depsSourceFilter = path: type:
    (craneLib.filterCargoSources path type)
    || (lib.hasSuffix ".proto" path)
    || (lib.hasSuffix "tauri.conf.json" path)
    || (lib.hasInfix "/capabilities/" path)
    || (lib.hasInfix "/.sqlx/" path)
    || (lib.hasSuffix ".sql" path);

  depsSrc = lib.cleanSourceWith {
    src = craneLib.path ../src-tauri;
    filter = depsSourceFilter;
  };

  cargoVendorDir = craneLib.vendorCargoDeps {
    src = craneLib.path ../src-tauri;
  };

  # Pre-compile cargo dependencies; cached as long as Cargo.lock is unchanged.
  # Features must match the main build.
  cargoArtifacts = craneLib.buildDepsOnly {
    inherit pname;
    inherit version buildInputs cargoVendorDir;
    src = depsSrc;
    nativeBuildInputs = cargoNativeBuildInputs;
    cargoExtraArgs = "--features custom-protocol";
    VERGEN_IDEMPOTENT = "true";
    SQLX_OFFLINE = "true";
  };

  # Prefetch pnpm dependencies.
  # Explicit pnpm_10 keeps fetchPnpmDeps and pnpmConfigHook on the same major version.
  pnpmDeps = fetchPnpmDeps {
    inherit pname version pnpm;
    src = ../.;
    fetcherVersion = 3;
    hash = "sha256-D7yf3x7OAeM3Wdjgo8TxeSCNoLxN3IfgqTzRoBdBwhc=";
  };

  # Prefetch pnpm dependencies for the new UI (separate pnpm project).
  newUiPnpmDeps = fetchPnpmDeps {
    pname = "defguard-client-new-ui";
    inherit version pnpm;
    src = ../new-ui;
    fetcherVersion = 3;
    hash = "sha256-yijiZxdKVpr5ZuZj1o3hqx5yhGgpUxicWwhE6h7xPsE=";
  };

  # Pre-build the new UI frontend so Tauri can serve it as WebviewUrl::App("compact/") and "full/".
  newUiDist = pkgs.stdenv.mkDerivation {
    pname = "defguard-client-new-ui";
    inherit version;
    src = ../new-ui;
    nativeBuildInputs = [pkgs.nodejs_24 pnpm pnpmConfigHook];
    pnpmDeps = newUiPnpmDeps;
    buildPhase = ''
      runHook preBuild
      pnpm tsc -b
      pnpm vite build --outDir "$out"
      # Create entry points for compact and full view windows.
      mkdir -p "$out"/compact "$out"/full
      cp "$out"/index.html "$out"/compact/
      cp "$out"/index.html "$out"/full/
      runHook postBuild
    '';
    installPhase = "true";
  };
in
  craneLib.mkCargoDerivation {
    inherit pname version buildInputs cargoArtifacts cargoVendorDir pnpmDeps newUiDist;

    src = ../.;

    nativeBuildInputs =
      cargoNativeBuildInputs
      ++ [
        pkgs.makeWrapper
        pkgs.wrapGAppsHook3
        pkgs.nodejs_24
        pnpm
        pnpmConfigHook
      ];

    # Pin CARGO_TARGET_DIR before crane's inheritCargoArtifacts hook runs so
    # extraction and tauri's cargo invocation both land in src-tauri/target.
    postUnpack = ''
      export CARGO_TARGET_DIR="$NIX_BUILD_TOP/$sourceRoot/src-tauri/target"
    '';

    # Required by mkCargoDerivation even when buildPhase is fully overridden.
    buildPhaseCargoCommand = "";

    preBuild = ''
      # Workspace-member build scripts were compiled in buildDepsOnly's source
      # tree (/build/source/) with that path baked in; remove them so cargo
      # recompiles them against the current tree.  Dep .rlib/.rmeta are kept.
      rm -rf src-tauri/target/release/build/defguard*
      rm -rf src-tauri/target/release/build/common*
      rm -rf src-tauri/target/release/.fingerprint/defguard*
      rm -rf src-tauri/target/release/.fingerprint/common*

      # tauri_build::build() reads OUT_DIR metadata written by tauri's own
      # build script during buildDepsOnly (pointing to /build/source/).
      # Remove tauri's build outputs and build-script-run fingerprints so
      # cargo re-runs the build script and refreshes OUT_DIR to the current
      # path.  libtauri*.rlib in deps/ is untouched.
      rm -rf src-tauri/target/release/build/tauri-*
      find src-tauri/target/release/.fingerprint \
        -maxdepth 1 -type d \( -name 'tauri-*' -o -name 'tauri_*' \) \
        -exec rm -f '{}/build-script-run' \;
    '';

    buildPhase = ''
      runHook preBuild

      # Build the old frontend and copy in the pre-built new UI.
      pnpm build
      cp -r ${newUiDist}/* dist/
      chmod -R u+w dist/

      # --config replaces the build section from tauri.linux.conf.json.
      pnpm tauri build \
        --config '{"build":{"beforeBuildCommand":""}}'

      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall

      # tauri always writes to src-tauri/target regardless of $CARGO_TARGET_DIR.
      local targetDir="src-tauri/target/release"

      mkdir -p $out/bin
      install -Dm755 "$targetDir/${pname}"         $out/bin/${pname}
      install -Dm755 "$targetDir/defguard-service" $out/bin/defguard-service
      install -Dm755 "$targetDir/defguard-cli"     $out/bin/defguard-cli
      install -Dm755 "$targetDir/dg"               $out/bin/dg

      mkdir -p $out/lib/${pname}
      cp -r src-tauri/resources $out/lib/${pname}/

      mkdir -p $out/share/applications
      cp ${desktopItem}/share/applications/* $out/share/applications/

      mkdir -p $out/share/icons/hicolor/{32x32,128x128}/apps
      install -Dm644 src-tauri/icons/32x32.png  $out/share/icons/hicolor/32x32/apps/${pname}.png
      install -Dm644 src-tauri/icons/128x128.png $out/share/icons/hicolor/128x128/apps/${pname}.png

      runHook postInstall
    '';

    preFixup = ''
      gappsWrapperArgs+=(
        --prefix PATH : ${lib.makeBinPath [pkgs.iproute2 pkgs.desktop-file-utils pkgs.lsb-release]}
        --suffix PATH : ${lib.makeBinPath [pkgs.openresolv]}
        --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [pkgs.libayatana-appindicator]}
      )
    '';

    VERGEN_IDEMPOTENT = "true";
    SQLX_OFFLINE = "true";
    doInstallCargoArtifacts = false;

    # passthru attrs are ignored by the build but addressable by external tools.
    # There are TWO pnpm lockfiles, each with its own pinned hash that must be kept
    # current when the corresponding lockfile changes:
    #   pnpmDeps       - root pnpm project (../pnpm-lock.yaml)
    #   newUiPnpmDeps  - new-ui pnpm project (../new-ui/pnpm-lock.yaml)
    # Any hash-refresh automation (e.g. an update-pnpm-hash workflow) must update both.
    passthru = {
      inherit pnpmDeps newUiPnpmDeps;
    };

    meta = with lib; {
      description = "Defguard VPN Client";
      homepage = "https://defguard.net";
      # license = licenses.gpl3Only;
      maintainers = with maintainers; [wojcik91];
      platforms = platforms.linux;
    };
  }
