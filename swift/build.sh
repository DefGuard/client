#!/bin/sh
set -e

DST="${PWD}/extension/BoringTun"
CARGO="${HOME}/.cargo/bin/cargo"
RUSTUP="${HOME}/.cargo/bin/rustup"

export MACOSX_DEPLOYMENT_TARGET=13.5

# Build BoringTun.

pushd boringtun

for TARGET in aarch64-apple-darwin x86_64-apple-darwin
do
    ${RUSTUP} target add "${TARGET}"
    ${CARGO} build --lib --locked --release --target ${TARGET}
done

# Create universal library.

mkdir -p target/universal/release
lipo -create \
    target/aarch64-apple-darwin/release/libboringtun.a \
    target/x86_64-apple-darwin/release/libboringtun.a \
    -output target/universal/release/libboringtun.a

rm -f -r target/uniffi
${CARGO} run --release --bin uniffi-bindgen -- \
    --xcframework --headers --modulemap --swift-sources \
    target/aarch64-apple-darwin/release/libboringtun.a target/uniffi

# Install BoringTun framework.

mkdir -p "${DST}"
cp -c target/uniffi/boringtun.swift "${DST}/"
rm -f -r "${DST}/boringtun.xcframework"
xcodebuild -create-xcframework \
    -library target/universal/release/libboringtun.a \
    -headers target/uniffi \
    -output ${DST}/boringtun.xcframework
cp -c target/uniffi/boringtunFFI.h "${DST}/"

popd

# Build VPNExtension.

# For release builds, TAURI_ENV_DEBUG is unset or 'false'
if [ "${TAURI_ENV_DEBUG}" = 'true' ]; then
    CONFIG=Debug
else
    CONFIG=Release
fi

echo "Building VPNExtension with configuration: ${CONFIG}"

# Check if building for Developer ID distribution
if [ "${DEVELOPER_ID_BUILD}" = 'true' ]; then
    echo "Building VPNExtension for Developer ID distribution..."
    xcodebuild -project extension/VPNExtension.xcodeproj -target VPNExtension -configuration ${CONFIG} \
        CODE_SIGN_IDENTITY="Developer ID Application: defguard sp. z o.o. (82GZ7KN29J)" \
        CODE_SIGN_STYLE=Manual \
        CODE_SIGN_ENTITLEMENTS="VPNExtension/VPNExtension.developerid.entitlements" \
        PROVISIONING_PROFILE_SPECIFIER="Defguard VPNExtension Mac DeveloperID" \
        OTHER_CODE_SIGN_FLAGS="--options runtime --timestamp" \
        build
else
    xcodebuild -project extension/VPNExtension.xcodeproj -target VPNExtension -configuration ${CONFIG} build
fi
