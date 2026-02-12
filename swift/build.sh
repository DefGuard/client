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
    target/aarch64-apple-darwin/release/libdefguard_boringtun.a \
    target/x86_64-apple-darwin/release/libdefguard_boringtun.a \
    -output target/universal/release/libdefguard_boringtun.a

rm -f -r target/uniffi
${CARGO} run --release --bin uniffi-bindgen -- \
    --xcframework --headers --modulemap --swift-sources \
    target/aarch64-apple-darwin/release/libdefguard_boringtun.a target/uniffi

# Install BoringTun framework.

mkdir -p "${DST}"
cp -c target/uniffi/defguard_boringtun.swift "${DST}/"
rm -f -r "${DST}/defguard_boringtun.xcframework"
xcodebuild -create-xcframework \
    -library target/universal/release/libdefguard_boringtun.a \
    -headers target/uniffi \
    -output ${DST}/defguard_boringtun.xcframework
cp -c target/uniffi/defguard_boringtunFFI.h "${DST}/"

popd

# Build VPNExtension.

# if [ "${TAURI_ENV_DEBUG}" = 'false' ]; then
    CONFIG=Release
# else
#     CONFIG=Debug
# fi
xcodebuild -project extension/VPNExtension.xcodeproj -target VPNExtension -configuration ${CONFIG} build
