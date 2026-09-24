# Client E2E tests

The client E2E suite runs the Linux Tauri client through WebDriver. It expects the
Defguard services to be running separately.

## Requirements

- Linux with WireGuard support.
- A dedicated, disposable Core instance. The tests create and remove users and devices and change network MFA settings.
- Core, Edge, and Gateway running and reachable from the test machine.
- The Gateway connected to the test network and reachable through WireGuard.
- An administrator account whose login does not require MFA. The password is supplied through `e2e/.env`.
- A dedicated deployment with exactly one test network named `e2e`. The connection checks use the first visible connect button.
- Root `defguard-service` running with `/var/run/defguard.socket` available.
- Rust/Cargo with the Tauri CLI, Node.js, `pnpm`, `just`, `WebKitWebDriver`, and `xvfb-run` available.
- `tauri-driver` available at `~/.cargo/bin/tauri-driver`, or at the path set by `TAURI_DRIVER`.

## Configure the test environment

Copy the example file and fill in the local deployment values:

```bash
cp e2e/.env.example e2e/.env
```

The variables are:

| Variable | Purpose |
| --- | --- |
| `CORE_URL` | Core HTTP base URL |
| `PROXY_URL` | Edge enrollment URL |
| `CORE_ADMIN_USER` | Core administrator username, default `admin` |
| `CORE_ADMIN_PASSWORD` | Core administrator password |
| `TEST_USERNAME` | Optional fixed test username |
| `GATEWAY_VPN_IP` | Gateway address used for the connectivity check |
| `NETWORK_ENDPOINT` | Endpoint advertised by the test network |
| `NETWORK_NAME` | Test network name, `e2e` |
| `NETWORK_ADDRESS` | Test network address |
| `NETWORK_PORT` | WireGuard port, default `50051` |
| `NETWORK_ALLOWED_IPS` | Allowed IP ranges for the test network |

Keep `e2e/.env` local and do not commit it.

## Run the suite

From the client repository root:

```bash
just e2e-build
just e2e-provision
just e2e-test
```

`just e2e` runs all three steps. Core must already be running before provisioning.

To run one spec directly:

```bash
cd e2e
NATIVE_DRIVER="$(command -v WebKitWebDriver)" \
  xvfb-run -a pnpm exec wdio run ./wdio.conf.ts \
  --spec=./tests/enrollment.spec.ts
```
