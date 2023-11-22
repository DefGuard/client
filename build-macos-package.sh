#!/bin/bash

set -e

DEVELOPER_ID_INSTALLER=$1

TARGET_DIRECTORY="./src-tauri/target"

build() {
    ARCHITECTURE=$1

    mkdir -p "${TARGET_DIRECTORY}/${ARCHITECTURE}/package"
    mkdir -p "${TARGET_DIRECTORY}/${ARCHITECTURE}/product"
    mkdir -p "${TARGET_DIRECTORY}/${ARCHITECTURE}/product-signed"

    APP_ROOT="${TARGET_DIRECTORY}/${ARCHITECTURE}/release/bundle/macos/defguard-client.app"

    pkgbuild \
        --analyze \
        --root ${APP_ROOT} \
        "${TARGET_DIRECTORY}/${ARCHITECTURE}/defguard-client.plist"
        
    PACKAGE_PATH="${TARGET_DIRECTORY}/${ARCHITECTURE}/package/defguard-${ARCHITECTURE}.pkg"

    pkgbuild \
        --identifier net.defguard \
        --root ${APP_ROOT} \
        --component-plist ${TARGET_DIRECTORY}/${ARCHITECTURE}/defguard-client.plist \
        --install-location "/Applications/defguard-client.app" \
        --scripts "./src-tauri/resources-macos/scripts" \
        "${PACKAGE_PATH}"

    productbuild \
        --package "${PACKAGE_PATH}" \
        "${TARGET_DIRECTORY}/${ARCHITECTURE}/product/defguard-${ARCHITECTURE}.pkg"

    productsign \
        --sign "Developer ID Installer: ${DEVELOPER_ID_INSTALLER}" \
        "${TARGET_DIRECTORY}/${ARCHITECTURE}/product/defguard-${ARCHITECTURE}.pkg" \
        "${TARGET_DIRECTORY}/${ARCHITECTURE}/product-signed/defguard-${ARCHITECTURE}.pkg"
}

build aarch64-apple-darwin
build x86_64-apple-darwin
