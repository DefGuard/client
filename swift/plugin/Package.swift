// swift-tools-version:5.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "defguard-vpn-plugin",
    platforms: [
        .macOS("13.5"),
        .iOS("15.6"),
    ],
    products: [
        // Products define the executables and libraries a package produces, and make them visible to other packages.
        .library(
            name: "defguard-vpn-plugin",
            type: .static,
            targets: ["defguard-vpn-plugin"])
    ],
    dependencies: [
        .package(url: "https://github.com/Brendonovich/swift-rs", from: "1.0.7")
    ],
    targets: [
        .target(
            name: "defguard-vpn-plugin",
            dependencies: [
                .product(
                    name: "SwiftRs",
                    package: "swift-rs"
                )
            ],
            path: "Sources")
    ]
)
