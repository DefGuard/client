// swift-tools-version:5.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "defguard-vpn-extension",
    platforms: [
        .macOS("13.5"),
        .iOS("15.6"),
    ],
    products: [
        // Products define the executables and libraries a package produces, and make them visible to other packages.
        .library(
            name: "defguard-vpn-extension",
            type: .static,
            targets: ["defguard-vpn-extension"])
    ],
    // dependencies: [
    //     .package(name: "Tauri", path: "../.tauri/tauri-api")
    // ],
    targets: [
        // Targets are the basic building blocks of a package. A target can define a module or a test suite.
        // Targets can depend on other targets in this package, and on products in packages this package depends on.
        .target(
            name: "defguard-vpn-extension",
            // dependencies: [
            //     .byName(name: "Tauri")
            // ],
            path: "Sources")
    ]
)
