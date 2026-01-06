// swift-tools-version:5.9
// Widget Extension for displaying location permission status on lock screen

import PackageDescription

let package = Package(
    name: "LocationWidget",
    platforms: [
        .iOS(.v17),  // iOS 17+ for latest Widget APIs
    ],
    products: [
        // Widget extensions are executables, not libraries
        .executable(
            name: "LocationWidget",
            targets: ["LocationWidget"]
        )
    ],
    dependencies: [],
    targets: [
        .executableTarget(
            name: "LocationWidget",
            path: "Sources",
            linkerSettings: [
                .linkedFramework("WidgetKit"),
                .linkedFramework("SwiftUI"),
                .linkedFramework("ActivityKit"),
            ]
        )
    ]
)
