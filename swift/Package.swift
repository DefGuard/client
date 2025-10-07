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
    dependencies: [
        .package(url: "https://github.com/Brendonovich/swift-rs", from: "1.0.7")
    ],
    targets: [
        .target(
            name: "defguard-vpn-extension",
            dependencies: [
                .product(
                    name: "SwiftRs",
                    package: "swift-rs"
                )
            ],
            path: "Sources")
    ]
)
