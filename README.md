 <p align="center">
    <img src="docs/header.png" alt="defguard">
 </p>

# defguard desktop client

Desktop client for managing WireGuard VPN connections (any WireGuard server and [defguard](https://github.com/DefGuard/defguard) instances).

![defguard desktop client](https://defguard.net/images/product/client/main-screen.png)

## Features

- Supports any WireGuard server
- Multi-platform - Linux, macOS & Windows
- Detailed network overview - see all details of your connection history and statistics with real-time charts and logs
- Multi-Factor Authentication with TOTP/Email & WireGuard PSK - Since WireGuard protocol doesn't support 2FA, most (if not all) available WireGuard clients use 2FA authorization to the "application" itself (not Wireguard tunnel). When using this client with [defguard VPN & SSO server](https://github.com/DefGuard/defguard) (which is <strong>free & open source</strong>) you will get <strong>real Multi-Factor Authentication using TOTP/Email codes + WireGuard Pre-shared session keys</strong>.
- Multiple instances & locations - When combining with [defguard](https://github.com/DefGuard/defguard) VPN & SSO you can have multiple defguard instances (sites/installations) and multiple Locations (VPN tunnels in that location/site) in <strong>one client</strong>! If you are an admin/devops - all your customers (instances) and all their tunnels (locations) can be in one place!
- Fast! - Built with Rust, [tauri](https://tauri.app/) and [React.js](https://react.dev/).

To learn more about the system see our [documentation](https://defguard.gitbook.io).

## Development

### Tauri requirements

Make sure to install prerequisites from [tauri](https://tauri.app/v1/guides/getting-started/prerequisites/).

### Install pnpm and node deps

```bash
pnpm install
```

### Dev server command

```bash
pnpm tauri dev
```

### Windows

Remove `default-run` line from `[package]` section in `Cargo.toml` to build the project.

# Legal

  - *defguard is not an official WireGuard project, and WireGuard is a registered trademark of Jason A. Donenfeld.*