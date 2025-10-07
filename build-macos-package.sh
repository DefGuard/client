#!/bin/bash

set -e

TARGET_DIRECTORY=$1
SCRIPTS_DIRECTORY=$2
APPLE_PACKAGE_SIGNING_IDENTITY=$3
KEYCHAIN=$4

mkdir -p "${TARGET_DIRECTORY}/package"
mkdir -p "${TARGET_DIRECTORY}/product"
mkdir -p "${TARGET_DIRECTORY}/product-signed"

APP_ROOT="${TARGET_DIRECTORY}/release/bundle/macos/defguard-client.app"

chmod -R 755 ${APP_ROOT}

pkgbuild \
    --analyze \
    --root ${APP_ROOT} \
    "${TARGET_DIRECTORY}/defguard-client.plist"

PACKAGE_PATH="${TARGET_DIRECTORY}/package/defguard.pkg"

pkgbuild \
    --identifier "net.defguard" \
    --root ${APP_ROOT} \
    --component-plist ${TARGET_DIRECTORY}/defguard-client.plist \
    --install-location "/Applications/defguard-client.app" \
    --scripts ${SCRIPTS_DIRECTORY} \
    "${PACKAGE_PATH}"

productbuild \
    --package "${PACKAGE_PATH}" \
    "${TARGET_DIRECTORY}/product/defguard.pkg"

productsign \
    --sign "${APPLE_PACKAGE_SIGNING_IDENTITY}" \
    --keychain "${KEYCHAIN}" \
    "${TARGET_DIRECTORY}/product/defguard.pkg" \
    "${TARGET_DIRECTORY}/product-signed/defguard.pkg"
