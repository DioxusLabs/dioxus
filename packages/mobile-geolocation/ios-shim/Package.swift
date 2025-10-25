// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "GeolocationShim",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "GeolocationShim",
            type: .static,
            targets: ["GeolocationShim"]
        ),
    ],
    targets: [
        .target(
            name: "GeolocationShim",
            dependencies: [],
            path: "Sources/GeolocationShim",
            publicHeadersPath: "../../include"
        ),
    ]
)

