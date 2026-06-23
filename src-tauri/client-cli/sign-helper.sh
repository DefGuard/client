#!/bin/bash
set -e

if [ "${TAURI_ENV_DEBUG}" = 'false' ]; then
    APP="src-tauri/target/release/bundle/macos/Defguard.app"
else
    APP="src-tauri/target/debug/bundle/macos/Defguard.app"
fi
CLI="${APP}/Contents/MacOS/defguard-cli"

codesign --force --options runtime \
         --entitlements src-tauri/Client.entitlements \
         --sign "${APPLE_SIGNING_IDENTITY}" \
         "${CLI}"

codesign --force --options runtime \
         --entitlements src-tauri/Client.entitlements \
         --sign "${APPLE_SIGNING_IDENTITY}" \
         "${APP}"
