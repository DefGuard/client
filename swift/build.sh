#!/bin/sh
set -e

DST="${PWD}/extension/BoringTun"
CARGO="${HOME}/.cargo/bin/cargo"
RUSTUP="${HOME}/.cargo/bin/rustup"
TARGET_DIR="${CARGO_TARGET_DIR:-./target}"

export MACOSX_DEPLOYMENT_TARGET=13.5

# Build BoringTun.
pushd boringtun

for TARGET in aarch64-apple-darwin x86_64-apple-darwin; do
    ${RUSTUP} target add "${TARGET}"
    ${CARGO} build --features=bindgen --lib --locked --release --target ${TARGET}
done

# Create universal library.
mkdir -p ${TARGET_DIR}/universal/release
lipo -create \
    ${TARGET_DIR}/aarch64-apple-darwin/release/libdefguard_boringtun.a \
    ${TARGET_DIR}/x86_64-apple-darwin/release/libdefguard_boringtun.a \
    -output ${TARGET_DIR}/universal/release/libdefguard_boringtun.a

rm -f -r ${TARGET_DIR}/uniffi
${CARGO} run --features=bindgen --release --bin uniffi-bindgen -- \
    --xcframework --headers --modulemap --swift-sources \
    ${TARGET_DIR}/aarch64-apple-darwin/release/libdefguard_boringtun.a ${TARGET_DIR}/uniffi

# Install BoringTun framework.
mkdir -p "${DST}"
cp -c ${TARGET_DIR}/uniffi/defguard_boringtun.swift "${DST}/"
rm -f -r "${DST}/defguard_boringtun.xcframework"
xcodebuild -create-xcframework \
    -library ${TARGET_DIR}/universal/release/libdefguard_boringtun.a \
    -headers ${TARGET_DIR}/uniffi \
    -output ${DST}/defguard_boringtun.xcframework
cp -c ${TARGET_DIR}/uniffi/defguard_boringtunFFI.h "${DST}/"

popd

# Build VPNExtension.
xcodebuild -project extension/VPNExtension.xcodeproj -target ${1:-VPNExtension} -configuration ${2:-Release} build
