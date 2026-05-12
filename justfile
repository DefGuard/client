set shell := ["powershell.exe", "-c"]

dev:
    npx concurrently \
        -n "NEW,OLD,TAURI" \
        "cd new-ui && pnpm dev" \
        "pnpm dev" \
        "cargo tauri dev"

build:
    cd new-ui; pnpm build
    pnpm build
    cargo tauri build
