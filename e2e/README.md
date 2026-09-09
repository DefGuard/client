# End-to-end tests

WebdriverIO drives the real client binary on Linux, macOS and Windows.

## Why the client needs a special build

macOS ships no WebDriver for WKWebView, so `tauri-driver` cannot be used there. Instead the
client is built with the `e2e` Cargo feature, which embeds a W3C WebDriver server in the app
(`tauri-plugin-wdio-webdriver`). The suite talks to that server directly, so no external
driver is needed on any platform. **The feature must never be enabled for a shipped build.**

## Running

```sh
# 1. Frontend
cd new-ui && pnpm install && pnpm build

# 2. Client, with the embedded WebDriver server
cargo tauri build --features e2e

# 3. Point the suite at your deployment
cd e2e && cp .env.example .env   # then fill in CORE_URL, PROXY_URL, secrets

# 4. Provision the core deployment (network + gateway check), then run
pnpm install
pnpm provision
pnpm test
```

Set `CLIENT_BINARY` to run against a binary elsewhere — a debug build, say:

```sh
cargo build --bin defguard-client --features custom-protocol,e2e
CLIENT_BINARY=${PWD}/../src-tauri/target/debug/defguard-client pnpm test
```

The suite runs the client against a throwaway profile under `${TMPDIR}/defguard-e2e` and
copies whatever the client logged into `e2e/logs/` before deleting it.

## When the run stops before any test

The suite checks two things before it starts the client, because both otherwise show up
only as `Failed to start embedded WebDriver ... did not become ready`, two minutes later:

- **The binary has no embedded WebDriver server.** It was built without `--features e2e`.
  A client like that starts up perfectly well and simply never answers.
- **Another Defguard client is already running.** Tauri's single-instance plugin makes the
  client under test hand its arguments to that one and exit, so no server ever starts. Quit
  the running client; the suite will not kill it for you, since on a developer machine it is
  a live VPN session.

Anything else, the client's own stderr is in the WebdriverIO output
(`captureBackendLogs`), and whatever it managed to log lands in `e2e/logs/`.

## Per-platform notes

There is no headless mode: the client renders through the platform's real window server.

- **Linux** — the app is a GTK binary, so a display is required. Wrap the run in
  `xvfb-run -a pnpm test` on a headless box. Connecting needs `defguard-service` running
  and the `wireguard` module loaded.
- **macOS** — needs a logged-in graphical session; SSH or a LaunchDaemon will not do.
  Connecting also needs the VPN system extension, which requires a signed build
  (`macos_installer` feature) and a one-time manual approval under System Settings >
  General > Login Items & Extensions. Until a host is approved, only the specs that never
  connect will pass.
- **Windows** — needs `defguard-service` running. Untested so far; the harness handles it
  but nothing has run there yet.
