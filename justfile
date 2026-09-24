set windows-shell := ["powershell.exe", "-c"]

dev:
    npx concurrently \
        -n "NEW,TAURI" \
        "cd new-ui && pnpm dev" \
        "cargo tauri dev"

# Run only the Tauri side with the local dev database and debug logging (Windows/PowerShell).
dev-tauri:
    cd src-tauri; $env:DATABASE_URL="sqlite:dev.db"; $env:DEFGUARD_CLIENT_DEV="1"; $env:DEFGUARD_CLIENT_LOG_LEVEL="debug"; cargo tauri dev

# Run only the web frontend dev server.
dev-web:
    cd new-ui; pnpm dev

build:
    cd new-ui; pnpm build
    cargo tauri build --config .\src-tauri\tauri.local.conf.json

# Build the Linux client and install both frontend dependency sets.
e2e-build:
    cd new-ui && pnpm install --frozen-lockfile && pnpm build
    cd e2e && pnpm install --frozen-lockfile
    cargo tauri build

# Provision the configured Core deployment. It must already be running.
e2e-provision:
    test -f e2e/.env || { echo "Missing e2e/.env; copy e2e/.env.example and fill it locally." >&2; exit 1; }
    cd e2e && pnpm provision

# Run the Linux WebDriver suite. Core, Edge, gateway, and defguard-service are external prerequisites.
e2e-test:
    test "$(uname -s)" = "Linux" || { echo "Client E2E tests require Linux." >&2; exit 1; }
    test -f e2e/.env || { echo "Missing e2e/.env; copy e2e/.env.example and fill it locally." >&2; exit 1; }
    test -S /var/run/defguard.socket || { echo "defguard-service is not running at /var/run/defguard.socket." >&2; exit 1; }
    test -x "${TAURI_DRIVER:-$HOME/.cargo/bin/tauri-driver}" || { echo "tauri-driver is missing; install it with: cargo install tauri-driver --locked" >&2; exit 1; }
    command -v WebKitWebDriver >/dev/null || { echo "WebKitWebDriver is missing; enter nix develop." >&2; exit 1; }
    command -v xvfb-run >/dev/null || { echo "xvfb-run is missing; enter nix develop." >&2; exit 1; }
    cd e2e && NATIVE_DRIVER="${NATIVE_DRIVER:-$(command -v WebKitWebDriver)}" xvfb-run -a pnpm test

e2e: e2e-build e2e-provision e2e-test
