set windows-shell := ["powershell.exe", "-c"]

dev:
    npx concurrently \
        -n "NEW,TAURI" \
        "cd new-ui && pnpm dev" \
        "cargo tauri dev"

build:
    cd new-ui; pnpm build
    cargo tauri build --config .\src-tauri\tauri.local.conf.json
