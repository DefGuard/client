#!/bin/bash

set -e

TARGET_DIRECTORY="./target"

mkdir -p "${TARGET_DIRECTORY}/macos/package"
mkdir -p "${TARGET_DIRECTORY}/macos/product"

APP_ROOT="${TARGET_DIRECTORY}/release/bundle/macos/defguard-client.app"

pkgbuild \
    --analyze \
    --root ${APP_ROOT} \
    "${TARGET_DIRECTORY}/macos/defguard-client.plist"
    
PACKAGE_PATH="${TARGET_DIRECTORY}/macos/package/defguard.pkg"

pkgbuild \
    --identifier net.defguard \
    --root ${APP_ROOT} \
    --component-plist ${TARGET_DIRECTORY}/macos/defguard-client.plist \
    --install-location "/Applications" \
    "${PACKAGE_PATH}"

productbuild \
    --resources "./resources-macos/resources" \
    --package "${PACKAGE_PATH}" \
    "${TARGET_DIRECTORY}/macos/product/defguard.pkg"
